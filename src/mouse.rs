use crate::{
    framebuffer,
    graphics::{Color, Draw, Offset, Point},
    layer,
    prelude::*,
    sync::{mpsc, OnceCell},
    window::Window,
};
use core::future::Future;
use enumflags2::{bitflags, BitFlags};

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

pub(crate) fn handler_task() -> impl Future<Output = Result<()>> {
    // Initialize MOUSE_EVENT_TX before co-task starts
    let (tx, mut rx) = mpsc::channel(100);
    MOUSE_EVENT_TX.init_once(|| tx);

    async move {
        let mut cursor_pos = Point::new(300, 200);
        let screen_info = *framebuffer::info();

        let mut window = Window::builder()
            .pos(cursor_pos)
            .size(MOUSE_CURSOR_SIZE)
            .transparent_color(Some(TRANSPARENT_COLOR))
            .height(layer::MOUSE_CURSOR_HEIGHT)
            .build()?;

        let cursor_layer_id = window.layer_id();
        draw(&mut window);
        window.flush().await?;

        let tx = layer::event_tx();

        // send dummy mouse event to notify cursor_layer_id
        tx.mouse_event(
            cursor_layer_id,
            MouseEvent {
                down: BitFlags::empty(),
                up: BitFlags::empty(),
                pos: cursor_pos,
                pos_diff: Offset::new(0, 0),
            },
        )
        .await?;

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
                window.move_to(cursor_pos).await?;
            }
            tx.mouse_event(
                cursor_layer_id,
                MouseEvent {
                    down,
                    up,
                    pos: cursor_pos,
                    pos_diff,
                },
            )
            .await?;
        }

        Ok(())
    }
}
