use crate::{
    font,
    graphics::{Color, Draw, Point, Rectangle, Size},
    keyboard::KeyboardEvent,
    layer::{self, Layer},
    prelude::*,
    sync::{mpsc, OnceCell},
    window::{self, Window},
};
use core::future::Future;
use futures_util::StreamExt;

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
            let window = Window::new(window_size)?;

            window.with_lock(|window| {
                window::draw_window(window, "Text Box Test");

                window::draw_text_box(
                    window,
                    Rectangle::new(
                        Point::new(4, 24),
                        Size::new(window_size.x - 8, window_size.y - 24 - 4),
                    ),
                );
            });

            let mut layer = Layer::new();
            let layer_id = layer.id();
            layer.set_draggable(true);
            layer.set_window(Some(window.clone()));
            layer.move_to(Point::new(350, 200));

            let tx = layer::event_tx();
            tx.register(layer)?;
            tx.set_height(layer_id, usize::MAX)?;
            tx.draw_layer(layer_id)?;

            let mut index = 0;
            let max_chars = (window_size.x - 16) / 8;
            while let Some(event) = rx.next().await {
                if event.ascii == '\0' {
                    continue;
                }

                window.with_lock(|window| {
                    if event.ascii == '\x08' && index > 0 {
                        index -= 1;
                        window.fill_rect(
                            Rectangle::new(Point::new(8 + 8 * index, 24 + 6), Size::new(8, 16)),
                            Color::WHITE,
                        );
                    } else if event.ascii >= ' ' && index < max_chars {
                        font::draw_char(
                            window,
                            Point::new(8 + 8 * index, 24 + 6),
                            event.ascii,
                            Color::BLACK,
                        );
                        index += 1;
                    }
                });
                tx.draw_layer(layer_id)?;
            }

            Ok::<(), Error>(())
        }
        .await;

        if let Err(err) = res {
            panic!("error occurred during handling text window event: {}", err);
        }
    }
}
