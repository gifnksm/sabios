use crate::{
    graphics::{
        frame_buffer, Buffer, BufferDrawer, Color, Draw, FrameBufferDrawer, Offset, Point,
        Rectangle, ScreenInfo, ShadowBuffer, Size,
    },
    keyboard::KeyboardEvent,
    mouse::{MouseButton, MouseEvent},
    prelude::*,
    sync::{mpsc, oneshot, MutexGuard, OnceCell},
    triple_buffer::Consumer,
    window::WindowEvent,
};
use alloc::{collections::BTreeMap, vec, vec::Vec};
use core::{
    future::Future,
    sync::atomic::{AtomicU32, Ordering},
};
use custom_debug_derive::Debug as CustomDebug;
use derivative::Derivative;

pub(crate) const DESKTOP_HEIGHT: usize = 0;
pub(crate) const CONSOLE_HEIGHT: usize = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct LayerId(u32);

impl LayerId {
    fn new() -> Self {
        static NEXT_ID: AtomicU32 = AtomicU32::new(0);
        LayerId(NEXT_ID.fetch_add(1, Ordering::Relaxed))
    }
}

#[derive(Derivative)]
#[derivative(Clone(clone_from = "true"))]
#[derive(CustomDebug)]
pub(crate) struct LayerBuffer {
    transparent_color: Option<Color>,
    #[debug(skip)]
    buffer: ShadowBuffer,
}

impl Draw for LayerBuffer {
    fn size(&self) -> Size<i32> {
        self.buffer.size()
    }

    fn draw(&mut self, p: Point<i32>, c: Color) {
        self.buffer.draw(p, c)
    }

    fn move_area(&mut self, offset: Point<i32>, src: Rectangle<i32>) {
        self.buffer.move_area(offset, src)
    }
}

impl LayerBuffer {
    pub(crate) fn new(size: Size<i32>, screen_info: ScreenInfo) -> Result<Self> {
        Ok(Self {
            transparent_color: None,
            buffer: ShadowBuffer::new_shadow(size, screen_info)?,
        })
    }

    pub(crate) fn set_transparent_color(&mut self, tc: Option<Color>) {
        self.transparent_color = tc;
    }

    fn draw_to<B>(
        &self,
        drawer: &mut BufferDrawer<B>,
        src_dst_offset: Offset<i32>,
        src_area: Rectangle<i32>,
    ) where
        B: Buffer,
    {
        if let Some(src_area) = src_area & self.buffer.area() {
            if let Some(tc) = self.transparent_color {
                for p in src_area.points() {
                    if let Some(c) = self.buffer.color_at(p) {
                        if tc != c {
                            drawer.draw(p + src_dst_offset, c);
                        }
                    }
                }
            } else {
                drawer.copy(src_dst_offset, &self.buffer, src_area);
            }
        }
    }
}

#[derive(Debug)]
pub(crate) struct Layer {
    id: LayerId,
    pos: Point<i32>,
    draggable: bool,
    consumer: Consumer<LayerBuffer>,
    tx: mpsc::Sender<WindowEvent>,
}

impl Layer {
    pub(crate) fn new(consumer: Consumer<LayerBuffer>, tx: mpsc::Sender<WindowEvent>) -> Self {
        Self {
            id: LayerId::new(),
            pos: Point::new(0, 0),
            draggable: false,
            consumer,
            tx,
        }
    }

    pub(crate) fn id(&self) -> LayerId {
        self.id
    }

    pub(crate) fn set_draggable(&mut self, draggable: bool) {
        self.draggable = draggable;
    }

    pub(crate) fn move_to(&mut self, pos: Point<i32>) {
        self.pos = pos;
    }

    fn load(&mut self) {
        self.consumer.load();
    }

    fn area(&self) -> Option<Rectangle<i32>> {
        let pos = self.pos;
        let size = self.consumer.buffer().size();
        Some(Rectangle { pos, size })
    }

    fn draw_to<B>(&self, drawer: &mut BufferDrawer<B>, dst_area: Rectangle<i32>)
    where
        B: Buffer,
    {
        let src_dst_offset = self.pos;
        let src_area = dst_area - self.pos;

        self.consumer
            .buffer()
            .draw_to(drawer, src_dst_offset, src_area);
    }

    fn send_event(&self, event: WindowEvent) -> Result<()> {
        self.tx.send(event)
    }
}

struct LayerManager {
    layers: BTreeMap<LayerId, Layer>,
    layer_stack: Vec<LayerId>,
    frame_buffer: MutexGuard<'static, FrameBufferDrawer>,
    back_buffer: ShadowBuffer,
}

impl LayerManager {
    fn new() -> Result<Self> {
        let frame_buffer = frame_buffer::lock_drawer();
        let back_buffer = ShadowBuffer::new_shadow(frame_buffer.size(), frame_buffer.info())?;
        Ok(Self {
            layers: BTreeMap::new(),
            layer_stack: vec![],
            frame_buffer,
            back_buffer,
        })
    }

    fn register(&mut self, layer: Layer) {
        let id = layer.id;
        self.layers.insert(id, layer);
    }

    fn draw_area(&mut self, dst_area: Rectangle<i32>) {
        // destructure `self` to avoid borrow checker errors
        let Self {
            layers,
            layer_stack,
            back_buffer,
            ..
        } = self;

        let layers = layer_stack.iter().filter_map(|id| layers.get(id));
        for layer in layers {
            layer.draw_to(back_buffer, dst_area);
        }

        self.finish_draw(dst_area);
    }

    fn draw_layer(&mut self, layer_id: LayerId) {
        if let Some(layer) = self.layers.get_mut(&layer_id) {
            layer.load();
        }

        (|| {
            // destructure `self` to avoid borrow checker errors
            let Self {
                layers,
                layer_stack,
                back_buffer,
                ..
            } = self;

            let dst_area = layers.get(&layer_id).and_then(Layer::area)?;
            let layers = layer_stack
                .iter()
                .skip_while(|id| **id != layer_id)
                .filter_map(|id| layers.get(id));
            for layer in layers {
                layer.draw_to(back_buffer, dst_area);
            }

            self.finish_draw(dst_area);

            Some(())
        })();
    }

    fn finish_draw(&mut self, area: Rectangle<i32>) {
        self.frame_buffer
            .copy(Offset::new(0, 0), &self.back_buffer, area);
    }

    fn move_to(&mut self, id: LayerId, pos: Point<i32>) {
        if let Some(layer) = self.layers.get_mut(&id) {
            let layer_id = layer.id();
            let old_area = layer.area();
            layer.move_to(pos);
            if let Some(old_area) = old_area {
                self.draw_area(old_area);
            }
            self.draw_layer(layer_id);
        }
    }

    fn move_relative(&mut self, id: LayerId, offset: Offset<i32>) {
        if let Some(layer) = self.layers.get_mut(&id) {
            let layer_id = layer.id();
            let old_area = layer.area();
            layer.move_to(layer.pos + offset);
            if let Some(old_area) = old_area {
                self.draw_area(old_area);
            }
            self.draw_layer(layer_id)
        }
    }

    fn height(&self) -> usize {
        self.layer_stack.len()
    }

    fn layer_height(&self, id: LayerId) -> Option<usize> {
        self.layer_stack.iter().position(|elem| *elem == id)
    }

    fn set_layer_height(&mut self, id: LayerId, height: usize) {
        if !self.layers.contains_key(&id) {
            return;
        }
        self.layer_stack.retain(|elem| *elem != id);
        let height = usize::min(height, self.layer_stack.len());
        self.layer_stack.insert(height, id);
    }

    // fn hide(&mut self, id: LayerId) {
    //     self.layer_stack.retain(|elem| *elem != id);
    // }

    fn layers_by_pos(&self, pos: Point<i32>) -> impl Iterator<Item = &Layer> {
        self.layer_stack
            .iter()
            .rev()
            .copied()
            .filter_map(move |layer_id| {
                self.layers.get(&layer_id).filter(|layer| {
                    layer
                        .area()
                        .map(|area| area.contains(&pos))
                        .unwrap_or(false)
                })
            })
    }

    fn notify_activated(&self, layer_id: LayerId) -> Result<()> {
        if let Some(layer) = self.layers.get(&layer_id) {
            layer.send_event(WindowEvent::Activated)?;
        }
        Ok(())
    }

    fn notify_deactivated(&self, layer_id: LayerId) -> Result<()> {
        if let Some(layer) = self.layers.get(&layer_id) {
            layer.send_event(WindowEvent::Deactivated)?;
        }
        Ok(())
    }

    fn notify_keyboard_event(&self, layer_id: LayerId, event: KeyboardEvent) -> Result<()> {
        if let Some(layer) = self.layers.get(&layer_id) {
            layer.send_event(WindowEvent::Keyboard(event))?;
        }
        Ok(())
    }
}

#[derive(Debug, Default)]
struct ActiveLayer {
    active_layer: Option<LayerId>,
    mouse_layer: Option<LayerId>,
}

impl ActiveLayer {
    fn new() -> Self {
        Self::default()
    }

    fn active_layer(&self) -> Option<LayerId> {
        self.active_layer
    }

    fn set_mouse_layer(&mut self, layer_manager: &mut LayerManager, layer_id: Option<LayerId>) {
        self.mouse_layer = layer_id;
        if let Some(layer_id) = self.mouse_layer {
            let height = layer_manager.height();
            layer_manager.set_layer_height(layer_id, height);
        }
        if let Some(layer_id) = self.active_layer {
            let height = self.active_height(layer_manager);
            layer_manager.set_layer_height(layer_id, height);
            layer_manager.draw_layer(layer_id);
        }
    }

    fn activate(&mut self, layer_manager: &mut LayerManager, layer_id: Option<LayerId>) {
        if self.active_layer == layer_id {
            return;
        }

        if let Some(layer_id) = self.active_layer {
            if let Err(err) = layer_manager.notify_deactivated(layer_id) {
                warn!("failed to notify_deactivated: {}", err);
            }
            layer_manager.draw_layer(layer_id);
        }

        self.active_layer = layer_id;

        if let Some(layer_id) = self.active_layer {
            let height = self.active_height(layer_manager);
            layer_manager.set_layer_height(layer_id, height);
            if let Err(err) = layer_manager.notify_activated(layer_id) {
                warn!("failed to notify_activated: {}", err);
            }
            layer_manager.draw_layer(layer_id);
        }
    }

    fn active_height(&self, layer_manager: &mut LayerManager) -> usize {
        self.mouse_layer
            .and_then(|layer_id| layer_manager.layer_height(layer_id))
            .map(|mouse_height| mouse_height - 1)
            .unwrap_or_else(|| layer_manager.height())
    }
}

#[derive(Debug)]
enum LayerEvent {
    Register {
        layer: Layer,
    },
    DrawLayer {
        layer_id: LayerId,
        tx: oneshot::Sender<()>,
    },
    MoveTo {
        layer_id: LayerId,
        pos: Point<i32>,
        tx: oneshot::Sender<()>,
    },
    SetHeight {
        layer_id: LayerId,
        height: usize,
    },
    // Hide {
    //     layer_id: LayerId,
    // },
    MouseEvent {
        cursor_layer_id: LayerId,
        event: MouseEvent,
        tx: oneshot::Sender<()>,
    },
    KeyboardEvent {
        event: KeyboardEvent,
        tx: oneshot::Sender<()>,
    },
}

static LAYER_EVENT_TX: OnceCell<mpsc::Sender<LayerEvent>> = OnceCell::uninit();

#[track_caller]
pub(crate) fn event_tx() -> EventSender {
    EventSender {
        tx: LAYER_EVENT_TX.get().clone(),
    }
}

#[derive(Debug, Clone)]
pub(crate) struct EventSender {
    tx: mpsc::Sender<LayerEvent>,
}

impl EventSender {
    fn send(&self, event: LayerEvent) -> Result<()> {
        self.tx.send(event)
    }

    pub(crate) fn register(&self, layer: Layer) -> Result<()> {
        self.send(LayerEvent::Register { layer })
    }

    pub(crate) async fn draw_layer(&self, layer_id: LayerId) -> Result<()> {
        let (tx, rx) = oneshot::channel();
        self.send(LayerEvent::DrawLayer { layer_id, tx })?;
        rx.await;
        Ok(())
    }

    pub(crate) async fn move_to(&self, layer_id: LayerId, pos: Point<i32>) -> Result<()> {
        let (tx, rx) = oneshot::channel();
        self.send(LayerEvent::MoveTo { layer_id, pos, tx })?;
        rx.await;
        Ok(())
    }

    pub(crate) fn set_height(&self, layer_id: LayerId, height: usize) -> Result<()> {
        self.send(LayerEvent::SetHeight { layer_id, height })
    }

    // pub(crate) fn hide(&self, layer_id: LayerId) -> Result<()> {
    //     self.send(LayerEvent::Hide { layer_id })
    // }

    pub(crate) async fn mouse_event(
        &self,
        cursor_layer_id: LayerId,
        event: MouseEvent,
    ) -> Result<()> {
        let (tx, rx) = oneshot::channel();
        self.send(LayerEvent::MouseEvent {
            cursor_layer_id,
            event,
            tx,
        })?;
        rx.await;
        Ok(())
    }

    pub(crate) async fn keyboard_event(&self, event: KeyboardEvent) -> Result<()> {
        let (tx, rx) = oneshot::channel();
        self.send(LayerEvent::KeyboardEvent { event, tx })?;
        rx.await;
        Ok(())
    }
}

pub(crate) fn handler_task() -> impl Future<Output = Result<()>> {
    // Initialize LAYER_EVENT_TX before co-task starts
    let (tx, mut rx) = mpsc::channel(100);
    LAYER_EVENT_TX.init_once(|| tx);

    async move {
        let mut lm = LayerManager::new()?;
        let mut am = ActiveLayer::new();

        let mut drag_layer_id = None;
        while let Some(event) = rx.next().await {
            match event {
                LayerEvent::Register { layer } => lm.register(layer),
                LayerEvent::DrawLayer { layer_id, tx } => {
                    lm.draw_layer(layer_id);
                    tx.send(());
                }
                LayerEvent::MoveTo { layer_id, pos, tx } => {
                    lm.move_to(layer_id, pos);
                    tx.send(());
                }
                LayerEvent::SetHeight { layer_id, height } => lm.set_layer_height(layer_id, height),
                // LayerEvent::Hide { layer_id } => lm.hide(layer_id),
                LayerEvent::MouseEvent {
                    cursor_layer_id,
                    event,
                    tx,
                } => {
                    am.set_mouse_layer(&mut lm, Some(cursor_layer_id));
                    let MouseEvent {
                        down,
                        up,
                        pos,
                        pos_diff,
                    } = event;
                    if up.contains(MouseButton::Left) {
                        drag_layer_id = None;
                    }
                    if let Some(layer_id) = drag_layer_id {
                        lm.move_relative(layer_id, pos_diff);
                    }
                    if down.contains(MouseButton::Left) {
                        drag_layer_id = lm
                            .layers_by_pos(pos)
                            .find(|layer| layer.id != cursor_layer_id)
                            .filter(|layer| layer.draggable)
                            .map(|layer| layer.id());
                        am.activate(&mut lm, drag_layer_id);
                    }
                    tx.send(());
                }
                LayerEvent::KeyboardEvent { event, tx } => {
                    if let Some(layer_id) = am.active_layer() {
                        if let Err(err) = lm.notify_keyboard_event(layer_id, event) {
                            warn!("failed to notify_keyboard_event: {}", err);
                        }
                    } else {
                        crate::println!("key push not handled: {:?}", event);
                    }
                    tx.send(());
                }
            }
        }

        Ok(())
    }
}
