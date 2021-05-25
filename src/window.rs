use crate::{
    font, framebuffer,
    graphics::{Color, Draw, Offset, Point, Rectangle, Size},
    layer::LayerBuffer,
    prelude::*,
};

#[derive(Debug)]
pub(crate) struct Window {
    buffer: LayerBuffer,
}

impl Window {
    pub(crate) fn new(size: Size<i32>) -> Result<Self> {
        let screen_info = *framebuffer::info();
        let buffer = LayerBuffer::new(size, screen_info)?;
        Ok(Self { buffer })
    }

    pub(crate) fn set_transparent_color(&mut self, tc: Option<Color>) {
        self.buffer.set_transparent_color(tc);
    }

    pub(crate) fn clone_buffer(&self, buffer: Option<LayerBuffer>) -> LayerBuffer {
        if let Some(mut buffer) = buffer {
            buffer.clone_from(&self.buffer);
            buffer
        } else {
            self.buffer.clone()
        }
    }
}

impl Draw for Window {
    fn size(&self) -> Size<i32> {
        self.buffer.size()
    }

    fn draw(&mut self, p: Point<i32>, c: Color) {
        self.buffer.draw(p, c);
    }

    fn move_area(&mut self, offset: Point<i32>, src: Rectangle<i32>) {
        self.buffer.move_area(offset, src);
    }
}

const CLOSE_BUTTON_WIDTH: usize = 16;
const CLOSE_BUTTON_HEIGHT: usize = 14;
const CLOSE_BUTTON: [[u8; CLOSE_BUTTON_WIDTH]; CLOSE_BUTTON_HEIGHT] = [
    *b"...............@",
    *b".:::::::::::::$@",
    *b".:::::::::::::$@",
    *b".:::@@::::@@::$@",
    *b".::::@@::@@:::$@",
    *b".:::::@@@@::::$@",
    *b".::::::@@:::::$@",
    *b".:::::@@@@::::$@",
    *b".::::@@::@@:::$@",
    *b".:::@@::::@@::$@",
    *b".:::::::::::::$@",
    *b".:::::::::::::$@",
    *b".$$$$$$$$$$$$$$@",
    *b"@@@@@@@@@@@@@@@@",
];

const EDGE_DARK: Color = Color::from_code(0x848484);
const EDGE_LIGHT: Color = Color::from_code(0xc6c6c6);
const BACKGROUND: Color = Color::from_code(0x000084);

pub(crate) fn draw_window<D>(drawer: &mut D, title: &str)
where
    D: Draw,
{
    let win_size = drawer.size();
    let (wx, wy) = (win_size.x, win_size.y);

    let data = &[
        ((0, 0), (wx, 1), EDGE_LIGHT),
        ((1, 1), (wx - 2, 1), Color::WHITE),
        ((0, 0), (1, wy), EDGE_LIGHT),
        ((1, 1), (1, wy - 2), Color::WHITE),
        ((wx - 2, 1), (1, wy - 2), EDGE_DARK),
        ((wx - 1, 0), (1, wy), Color::BLACK),
        ((2, 2), (wx - 4, wy - 4), EDGE_LIGHT),
        ((3, 3), (wx - 6, 18), BACKGROUND),
        ((1, wy - 2), (wx - 2, 1), EDGE_DARK),
        ((0, wy - 1), (wx, 1), Color::BLACK),
    ];

    for (pos, size, color) in data {
        drawer.fill_rect(
            Rectangle::new(Point::new(pos.0, pos.1), Size::new(size.0, size.1)),
            *color,
        );
    }

    font::draw_str(drawer, Point::new(24, 4), title, Color::WHITE);

    for (y, row) in (0..).zip(CLOSE_BUTTON) {
        for (x, ch) in (0..).zip(row) {
            let c = match ch {
                b'@' => Color::BLACK,
                b'$' => EDGE_DARK,
                b':' => EDGE_LIGHT,
                b'.' => Color::WHITE,
                _ => panic!("invalid char: {}", ch),
            };
            drawer.draw(Point::new(wx - 5 - CLOSE_BUTTON_WIDTH as i32 + x, 5 + y), c);
        }
    }
}

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
