use crate::{
    font,
    graphics::{Color, Draw, Offset, Point, Rectangle, Size},
    keyboard::KeyboardEvent,
    prelude::*,
    sync::{mpsc, OnceCell},
    timer,
    window::{self, Window},
};
use core::future::Future;
use futures_util::{select_biased, FutureExt, StreamExt};

static KEYBOARD_EVENT_TX: OnceCell<mpsc::Sender<KeyboardEvent>> = OnceCell::uninit();

pub(crate) fn sender() -> mpsc::Sender<KeyboardEvent> {
    KEYBOARD_EVENT_TX.get().clone()
}

pub(crate) fn handler_task() -> impl Future<Output = ()> {
    // Initialize KEYBOARD_EVENT_TX before co-task starts
    let (tx, mut rx) = mpsc::channel(100);
    KEYBOARD_EVENT_TX.init_once(|| tx);

    async move {
        let res = async {
            let window_size = Size::new(160, 52);
            let mut window = Window::builder()
                .size(window_size)
                .pos(Point::new(350, 200))
                .draggable(true)
                .height(usize::MAX)
                .build()?;

            window::draw_window(&mut window, "Text Box Test");
            window::draw_text_box(
                &mut window,
                Rectangle::new(
                    Point::new(4, 24),
                    Size::new(window_size.x - 8, window_size.y - 24 - 4),
                ),
            );
            window.flush().await?;

            let mut index = 0;
            let max_chars = (window_size.x - 16) / 8;
            let pos = |index| Point::new(8 + 8 * index, 24 + 6);
            let draw_cursor = |window: &mut Window, index, visible| {
                let color = if visible { Color::BLACK } else { Color::WHITE };
                let pos = pos(index) - Offset::new(0, 1);
                window.fill_rect(Rectangle::new(pos, Size::new(7, 15)), color);
            };
            let mut interval = timer::lapic::interval(0, 50)?;
            let mut cursor_visible = true;
            loop {
                select_biased! {
                    event = rx.next().fuse() => {
                        let event = match event {
                            Some(event) => event,
                            None => break,
                        };
                        if event.ascii == '\0' {
                            continue;
                        }

                        if event.ascii == '\x08' && index > 0 {
                            draw_cursor(&mut window, index, false);
                            index -= 1;
                            window
                                .fill_rect(Rectangle::new(pos(index), Size::new(8, 16)), Color::WHITE);
                            draw_cursor(&mut window, index, cursor_visible);
                        } else if event.ascii >= ' ' && index < max_chars {
                            draw_cursor(&mut window, index, false);
                            font::draw_char(&mut window, pos(index), event.ascii, Color::BLACK);
                            index += 1;
                            draw_cursor(&mut window, index, cursor_visible);
                        }
                    }
                    timeout = interval.next().fuse() => {
                        let _timeout = match timeout {
                            Some(Ok(timeout)) => timeout,
                            _ => break,
                        };
                        cursor_visible = !cursor_visible;
                            draw_cursor(&mut window, index, cursor_visible);
                    }
                }
                window.flush().await?;
            }

            Ok::<(), Error>(())
        }
        .await;

        if let Err(err) = res {
            panic!("error occurred during handling text window event: {}", err);
        }
    }
}
