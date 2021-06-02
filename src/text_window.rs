use crate::{
    font,
    framed_window::{FramedWindow, FramedWindowEvent},
    graphics::{Color, Draw, Offset, Point, Rectangle, Size},
    prelude::*,
    timer,
};
use futures_util::{select_biased, FutureExt, StreamExt};

pub(crate) async fn handler_task() {
    let res = async {
        let window_size = Size::new(160, 24);
        let mut window = FramedWindow::builder("Text Box Test".into())
            .size(window_size)
            .pos(Point::new(350, 200))
            .build()?;

        draw_text_box(
            &mut window,
            Rectangle::new(Point::new(0, 0), Size::new(window_size.x, window_size.y)),
        );
        window.flush().await?;

        let mut index = 0;
        let max_chars = (window_size.x - 8) / 8 - 1;
        let pos = |index| Point::new(4 + 8 * index, 6);
        let draw_cursor = |window: &mut FramedWindow, index, visible| {
            let color = if visible { Color::BLACK } else { Color::WHITE };
            let pos = pos(index) - Offset::new(0, 1);
            window.fill_rect(Rectangle::new(pos, Size::new(7, 15)), color);
        };
        let mut interval = timer::lapic::interval(0, 50)?;
        let mut cursor_visible = true;
        'outer: loop {
            select_biased! {
                event = window.recv_event().fuse() => {
                    let event = match event {
                        Some(Ok(event)) => event,
                        Some(Err(err)) => bail!(err),
                        None => break 'outer,
                    };
                    match event {
                        FramedWindowEvent::Keyboard(event) => {
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

const EDGE_DARK: Color = Color::from_code(0x848484);
const EDGE_LIGHT: Color = Color::from_code(0xc6c6c6);

pub(crate) fn draw_text_box<D>(drawer: &mut D, area: Rectangle<i32>)
where
    D: Draw,
{
    // fill main box
    drawer.fill_rect(
        Rectangle::new(area.pos + Offset::new(1, 1), area.size - Offset::new(2, 2)),
        Color::WHITE,
    );

    // draw border lines
    drawer.fill_rect(
        Rectangle::new(area.pos, Size::new(area.size.x, 1)),
        EDGE_DARK,
    );
    drawer.fill_rect(
        Rectangle::new(area.pos, Size::new(1, area.size.y)),
        EDGE_DARK,
    );
    drawer.fill_rect(
        Rectangle::new(
            area.pos + Offset::new(0, area.size.y),
            Size::new(area.size.x, 1),
        ),
        EDGE_LIGHT,
    );
    drawer.fill_rect(
        Rectangle::new(
            area.pos + Offset::new(area.size.x, 0),
            Size::new(1, area.size.y),
        ),
        EDGE_LIGHT,
    );
}
