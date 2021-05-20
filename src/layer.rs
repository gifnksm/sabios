use crate::{
    framebuffer,
    graphics::{Point, Rectangle},
    prelude::*,
    sync::{mpsc, Mutex, OnceCell},
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

    fn draw_to(&self, drawer: &mut framebuffer::Drawer, dst_area: Rectangle<i32>) {
        if let Some(window) = self.window.as_ref() {
            window.lock().draw_to(drawer, self.pos, dst_area - self.pos)
        }
    }
}

#[derive(Debug)]
pub(crate) struct LayerManager {
    layers: BTreeMap<LayerId, Layer>,
    layer_stack: Vec<LayerId>,
}

impl LayerManager {
    fn new() -> Self {
        Self {
            layers: BTreeMap::new(),
            layer_stack: vec![],
        }
    }

    fn register(&mut self, layer: Layer) {
        let id = layer.id;
        self.layers.insert(id, layer);
    }

    fn draw_area(&self, drawer: &mut framebuffer::Drawer, dst_area: Rectangle<i32>) {
        let layers = self.layer_stack.iter().filter_map(|id| self.layers.get(id));
        for layer in layers {
            layer.draw_to(drawer, dst_area);
        }
    }

    fn draw_layer(&self, drawer: &mut framebuffer::Drawer, layer_id: LayerId) {
        (|| {
            let dst_area = self.layers.get(&layer_id).and_then(Layer::area)?;
            let layers = self
                .layer_stack
                .iter()
                .skip_while(|id| **id != layer_id)
                .filter_map(|id| self.layers.get(id));
            for layer in layers {
                layer.draw_to(drawer, dst_area);
            }

            Some(())
        })();
    }

    fn move_to(&mut self, drawer: &mut framebuffer::Drawer, id: LayerId, pos: Point<i32>) {
        if let Some(layer) = self.layers.get_mut(&id) {
            let layer_id = layer.id();
            let old_area = layer.area();
            layer.move_to(pos);
            if let Some(old_area) = old_area {
                self.draw_area(drawer, old_area);
            }
            self.draw_layer(drawer, layer_id);
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
}

#[derive(Debug)]
enum LayerEvent {
    Register { layer: Layer },
    DrawLayer { layer_id: LayerId },
    MoveTo { layer_id: LayerId, pos: Point<i32> },
    SetHeight { layer_id: LayerId, height: usize },
    // Hide {
    //     layer_id: LayerId,
    // },
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
}

pub(crate) fn handler_task() -> impl Future<Output = ()> {
    // Initialize LAYER_EVENT_TX before co-task starts
    let (tx, mut rx) = mpsc::channel(100);
    LAYER_EVENT_TX.init_once(|| tx);

    async move {
        let res = async {
            let mut layer_manager = LayerManager::new();
            while let Some(event) = rx.next().await {
                match event {
                    LayerEvent::Register { layer } => layer_manager.register(layer),
                    LayerEvent::DrawLayer { layer_id } => {
                        let mut framebuffer = framebuffer::lock_drawer();
                        layer_manager.draw_layer(&mut *framebuffer, layer_id);
                    }
                    LayerEvent::MoveTo { layer_id, pos } => {
                        let mut framebuffer = framebuffer::lock_drawer();
                        layer_manager.move_to(&mut *framebuffer, layer_id, pos);
                    }
                    LayerEvent::SetHeight { layer_id, height } => {
                        layer_manager.set_height(layer_id, height)
                    } // LayerEvent::Hide { layer_id } => layer_manager.hide(layer_id),
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
