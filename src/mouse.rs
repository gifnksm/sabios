use crate::{
    framebuffer,
    graphics::{Color, Draw, Offset, Point},
    layer::{self, Layer},
    prelude::*,
    sync::{mpsc, OnceCell},
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
    displacement: Offset<i32>,
}

static MOUSE_EVENT_TX: OnceCell<mpsc::Sender<MouseEvent>> = OnceCell::uninit();

pub(crate) extern "C" fn observer(displacement_x: i8, displacement_y: i8) {
    let event = MouseEvent {
        displacement: Offset::new(i32::from(displacement_x), i32::from(displacement_y)),
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
            window.with_lock(|window| {
                window.set_transparent_color(Some(TRANSPARENT_COLOR));
                draw(window);
            });

            let mut cursor_pos = Point::new(300, 200);
            let screen_info = *framebuffer::info();

            let mut layer = Layer::new();
            let layer_id = layer.id();
            layer.set_window(Some(window));
            layer.move_to(cursor_pos);

            let tx = layer::event_tx();
            tx.register(layer)?;
            tx.set_height(layer_id, layer::MOUSE_CURSOR_HEIGHT)?;
            tx.draw_layer(layer_id)?;

            while let Some(event) = rx.next().await {
                if let Some(pos) = (cursor_pos + event.displacement).clamp(screen_info.area()) {
                    cursor_pos = pos;
                    tx.move_to(layer_id, pos)?;
                }
            }

            Ok::<(), Error>(())
        }
        .await;
        if let Err(err) = res {
            panic!("error occurred during handling mouse cursor event: {}", err);
        }
    }
}
