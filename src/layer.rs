use crate::{
    framebuffer,
    graphics::{Point, Vector2d},
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
pub(crate) const MOUSE_CURSOR_HEIGHT: usize = 2;

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

    pub(crate) fn move_relative(&mut self, diff: Vector2d<i32>) {
        self.pos += diff;
    }

    fn draw(&self, drawer: &mut framebuffer::Drawer) {
        if let Some(window) = self.window.as_ref() {
            window.lock().draw_to(drawer, self.pos)
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

    fn draw(&self, drawer: &mut framebuffer::Drawer) {
        for id in &self.layer_stack {
            if let Some(layer) = self.layers.get(&id) {
                layer.draw(drawer);
            }
        }
    }

    // fn move_to(&mut self, id: LayerId, pos: Point<i32>) {
    //     if let Some(layer) = self.layers.get_mut(&id) {
    //         layer.move_to(pos);
    //     }
    // }

    fn move_relative(&mut self, id: LayerId, diff: Vector2d<i32>) {
        if let Some(layer) = self.layers.get_mut(&id) {
            layer.move_relative(diff);
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
pub(crate) enum LayerEvent {
    Register {
        layer: Layer,
    },
    Draw {
        bench: bool,
    },
    // MoveTo {
    //     layer_id: LayerId,
    //     pos: Point<i32>,
    // },
    MoveRelative {
        layer_id: LayerId,
        diff: Vector2d<i32>,
    },
    SetHeight {
        layer_id: LayerId,
        height: usize,
    },
    // Hide {
    //     layer_id: LayerId,
    // },
}

static LAYER_EVENT_TX: OnceCell<mpsc::Sender<LayerEvent>> = OnceCell::uninit();

pub(crate) fn event_tx() -> mpsc::Sender<LayerEvent> {
    LAYER_EVENT_TX.get().clone()
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
                    LayerEvent::Draw { bench } => {
                        use crate::timer::lapic;
                        let mut framebuffer = framebuffer::lock_drawer();
                        lapic::start();
                        layer_manager.draw(&mut *framebuffer);
                        let elapsed = lapic::elapsed();
                        lapic::stop();
                        if bench {
                            crate::println!("{}", elapsed);
                        }
                    }
                    // LayerEvent::MoveTo { layer_id, pos } => layer_manager.move_to(layer_id, pos),
                    LayerEvent::MoveRelative { layer_id, diff } => {
                        layer_manager.move_relative(layer_id, diff)
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
