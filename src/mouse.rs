use crate::{
    graphics::{Color, Draw, Point, Vector2d},
    layer::{self, Layer, LayerEvent},
    prelude::*,
    sync::{mpsc, once_cell::OnceCell},
    window::Window,
};
use core::future::Future;
use futures_util::StreamExt as _;

const TRANSPARENT_COLOR: Color = Color::RED;
const MOUSE_CURSOR_WIDTH: usize = 15;
const MOUSE_CURSOR_HEIGHT: usize = 24;
const MOUSE_CURSOR_SIZE: Point<i32> =
    Point::new(MOUSE_CURSOR_WIDTH as i32, MOUSE_CURSOR_HEIGHT as i32);

const MOUSE_CURSOR_SHAPE: [[u8; MOUSE_CURSOR_WIDTH]; MOUSE_CURSOR_HEIGHT] = [
    *b"@              ",
    *b"@@             ",
    *b"@.@            ",
    *b"@..@           ",
    *b"@...@          ",
    *b"@....@         ",
    *b"@.....@        ",
    *b"@......@       ",
    *b"@.......@      ",
    *b"@........@     ",
    *b"@.........@    ",
    *b"@..........@   ",
    *b"@...........@  ",
    *b"@............@ ",
    *b"@......@@@@@@@@",
    *b"@......@       ",
    *b"@....@@.@      ",
    *b"@...@ @.@      ",
    *b"@..@   @.@     ",
    *b"@.@    @.@     ",
    *b"@@      @.@    ",
    *b"@       @.@    ",
    *b"         @.@   ",
    *b"         @@@   ",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct MouseEvent {
    displacement: Vector2d<i32>,
}

static MOUSE_EVENT_TX: OnceCell<mpsc::Sender<MouseEvent>> = OnceCell::uninit();

pub(crate) extern "C" fn observer(displacement_x: i8, displacement_y: i8) {
    let event = MouseEvent {
        displacement: Vector2d::new(i32::from(displacement_x), i32::from(displacement_y)),
    };

    let res = MOUSE_EVENT_TX.try_get().and_then(|tx| tx.send(event));

    if let Err(err) = res {
        error!("failed to enqueue to the queue: {}", err);
    }
}

fn draw(drawer: &mut dyn Draw) {
    for (dy, row) in (0..).zip(MOUSE_CURSOR_SHAPE) {
        for (dx, ch) in (0..).zip(row) {
            let p = Point::new(dx, dy);
            match ch {
                b'@' => drawer.draw(p, Color::BLACK),
                b'.' => drawer.draw(p, Color::WHITE),
                b' ' => drawer.draw(p, TRANSPARENT_COLOR),
                _ => {}
            }
        }
    }
}

pub(crate) fn handler_task() -> impl Future<Output = ()> {
    // Initialize MOUSE_EVENT_TX before co-task starts
    let (tx, mut rx) = mpsc::channel(100);
    MOUSE_EVENT_TX.init_once(|| tx);

    async move {
        let res = async {
            let window = Window::new(MOUSE_CURSOR_SIZE);
            {
                let mut window = window.lock();
                window.set_transparent_color(Some(TRANSPARENT_COLOR));
            }

            {
                let drawer = window.lock().drawer();
                let mut drawer = drawer.lock();
                draw(&mut *drawer);
            }

            let mut layer = Layer::new();
            let layer_id = layer.id();
            layer.set_window(Some(window));
            layer.move_to(Point::new(300, 200));

            let tx = layer::event_tx();
            tx.send(LayerEvent::Register { layer })?;
            tx.send(LayerEvent::SetHeight {
                layer_id,
                height: layer::MOUSE_CURSOR_HEIGHT,
            })?;
            tx.send(LayerEvent::Draw { bench: true })?;

            while let Some(event) = rx.next().await {
                tx.send(LayerEvent::MoveRelative {
                    layer_id,
                    diff: event.displacement,
                })?;
                tx.send(LayerEvent::Draw { bench: true })?;
            }

            Ok::<(), Error>(())
        }
        .await;
        if let Err(err) = res {
            panic!("error occurred during handling mouse cursor event: {}", err);
        }
    }
}
