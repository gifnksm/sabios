use crate::{
    buffer_drawer::{Buffer, BufferDrawer, ShadowBuffer},
    framebuffer,
    graphics::{Draw, Offset, Point, Rectangle},
    mouse::{MouseButton, MouseEvent},
    prelude::*,
    sync::{mpsc, Mutex, MutexGuard, OnceCell},
    window::Window,
};
use alloc::{collections::BTreeMap, sync::Arc, vec, vec::Vec};
use core::{
    future::Future,
    sync::atomic::{AtomicU32, Ordering},
};
use futures_util::StreamExt as _;

pub(crate) const DESKTOP_HEIGHT: usize = 0;
pub(crate) const CONSOLE_HEIGHT: usize = 1;
pub(crate) const MAIN_WINDOW_ID: usize = 2;
pub(crate) const MOUSE_CURSOR_HEIGHT: usize = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct LayerId(u32);

impl LayerId {
    fn new() -> Self {
        static NEXT_ID: AtomicU32 = AtomicU32::new(0);
        LayerId(NEXT_ID.fetch_add(1, Ordering::Relaxed))
    }
}

#[derive(Debug)]
pub(crate) struct Layer {
    id: LayerId,
    pos: Point<i32>,
    window: Option<Arc<Mutex<Window>>>,
}

impl Layer {
    pub(crate) fn new() -> Self {
        Self {
            id: LayerId::new(),
            pos: Point::new(0, 0),
            window: None,
        }
    }

    pub(crate) fn id(&self) -> LayerId {
        self.id
    }

    pub(crate) fn set_window(&mut self, window: Option<Arc<Mutex<Window>>>) {
        self.window = window;
    }

    // fn window(&mut self) -> Option<Arc<Mutex<Window>>> {
    //     self.window.clone()
    // }

    pub(crate) fn move_to(&mut self, pos: Point<i32>) {
        self.pos = pos;
    }

    fn area(&self) -> Option<Rectangle<i32>> {
        let pos = self.pos;
        let size = self.window.as_ref()?.lock().size();
        Some(Rectangle { pos, size })
    }

    fn draw_to<B>(&self, drawer: &mut BufferDrawer<B>, dst_area: Rectangle<i32>)
    where
        B: Buffer,
    {
        if let Some(window) = self.window.as_ref() {
            window.lock().draw_to(drawer, self.pos, dst_area - self.pos)
        }
    }
}

pub(crate) struct LayerManager {
    layers: BTreeMap<LayerId, Layer>,
    layer_stack: Vec<LayerId>,
    framebuffer: MutexGuard<'static, framebuffer::Drawer>,
    back_buffer: ShadowBuffer,
}

impl LayerManager {
    fn new() -> Result<Self> {
        let framebuffer = framebuffer::lock_drawer();
        let back_buffer = ShadowBuffer::new_shadow(framebuffer.size(), framebuffer.info())?;
        Ok(Self {
            layers: BTreeMap::new(),
            layer_stack: vec![],
            framebuffer,
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
        self.framebuffer
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

    fn set_height(&mut self, id: LayerId, height: usize) {
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
}

#[derive(Debug)]
enum LayerEvent {
    Register {
        layer: Layer,
    },
    DrawLayer {
        layer_id: LayerId,
    },
    MoveTo {
        layer_id: LayerId,
        pos: Point<i32>,
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
    },
}

static LAYER_EVENT_TX: OnceCell<mpsc::Sender<LayerEvent>> = OnceCell::uninit();

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

    pub(crate) fn draw_layer(&self, layer_id: LayerId) -> Result<()> {
        self.send(LayerEvent::DrawLayer { layer_id })
    }

    pub(crate) fn move_to(&self, layer_id: LayerId, pos: Point<i32>) -> Result<()> {
        self.send(LayerEvent::MoveTo { layer_id, pos })
    }

    pub(crate) fn set_height(&self, layer_id: LayerId, height: usize) -> Result<()> {
        self.send(LayerEvent::SetHeight { layer_id, height })
    }

    // pub(crate) fn hide(&self, layer_id: LayerId) -> Result<()> {
    //     self.send(LayerEvent::Hide { layer_id })
    // }

    pub(crate) fn mouse_event(&self, cursor_layer_id: LayerId, event: MouseEvent) -> Result<()> {
        self.send(LayerEvent::MouseEvent {
            cursor_layer_id,
            event,
        })
    }
}

pub(crate) fn handler_task() -> impl Future<Output = ()> {
    // Initialize LAYER_EVENT_TX before co-task starts
    let (tx, mut rx) = mpsc::channel(100);
    LAYER_EVENT_TX.init_once(|| tx);

    async move {
        let res = async {
            let mut lm = LayerManager::new()?;

            let mut drag_layer_id = None;
            while let Some(event) = rx.next().await {
                match event {
                    LayerEvent::Register { layer } => lm.register(layer),
                    LayerEvent::DrawLayer { layer_id } => lm.draw_layer(layer_id),
                    LayerEvent::MoveTo { layer_id, pos } => lm.move_to(layer_id, pos),
                    LayerEvent::SetHeight { layer_id, height } => lm.set_height(layer_id, height),
                    // LayerEvent::Hide { layer_id } => lm.hide(layer_id),
                    LayerEvent::MouseEvent {
                        cursor_layer_id,
                        event,
                    } => {
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
                                .map(|layer| layer.id());
                        }
                    }
                }
            }

            Ok::<(), Error>(())
        }
        .await;
        if let Err(err) = res {
            panic!("error occurred during handling layer event: {}", err);
        }
    }
}
