use crate::{
    framebuffer,
    graphics::{Color, Draw, Offset, Point},
    layer::{self, Layer, LayerDrawer},
    prelude::*,
    sync::{mpsc, OnceCell},
    window::Window,
};
use core::future::Future;
use enumflags2::{bitflags, BitFlags};
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

#[bitflags]
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MouseButton {
    Left = 0b001,
    Right = 0b010,
    Middle = 0b100,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RawMouseEvent {
    buttons: BitFlags<MouseButton>,
    displacement: Offset<i32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct MouseEvent {
    pub(crate) down: BitFlags<MouseButton>,
    pub(crate) up: BitFlags<MouseButton>,
    pub(crate) pos: Point<i32>,
    pub(crate) pos_diff: Offset<i32>,
}

static MOUSE_EVENT_TX: OnceCell<mpsc::Sender<RawMouseEvent>> = OnceCell::uninit();

pub(crate) extern "C" fn observer(buttons: u8, displacement_x: i8, displacement_y: i8) {
    let buttons = BitFlags::<MouseButton>::from_bits_truncate(buttons);
    let event = RawMouseEvent {
        buttons,
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
            let mut window = Window::new(MOUSE_CURSOR_SIZE)?;
            window.set_transparent_color(Some(TRANSPARENT_COLOR));
            draw(&mut window);

            let mut cursor_pos = Point::new(300, 200);
            let screen_info = *framebuffer::info();

            let mut layer = Layer::new();
            let layer_id = layer.id();
            layer.move_to(cursor_pos);

            let tx = layer::event_tx();
            let mut drawer = LayerDrawer::new();
            tx.register(layer)?;
            tx.set_height(layer_id, layer::MOUSE_CURSOR_HEIGHT)?;
            drawer.draw(layer_id, &window).await?;

            let mut buttons = BitFlags::empty();
            while let Some(event) = rx.next().await {
                let prev_cursor_pos = cursor_pos;
                let prev_buttons = buttons;

                if let Some(pos) = (cursor_pos + event.displacement).clamp(screen_info.area()) {
                    cursor_pos = pos;
                }
                buttons = event.buttons;

                let down = buttons & !prev_buttons;
                let up = prev_buttons & !buttons;
                let pos_diff = cursor_pos - prev_cursor_pos;

                if prev_cursor_pos != cursor_pos {
                    tx.move_to(layer_id, cursor_pos)?;
                }
                tx.mouse_event(
                    layer_id,
                    MouseEvent {
                        down,
                        up,
                        pos: cursor_pos,
                        pos_diff,
                    },
                )?;
            }

            Ok::<(), Error>(())
        }
        .await;
        if let Err(err) = res {
            panic!("error occurred during handling mouse cursor event: {}", err);
        }
    }
}
