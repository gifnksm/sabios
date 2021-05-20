use crate::{
    buffer_drawer::ShadowBuffer,
    font, framebuffer,
    graphics::{Color, Draw, Point, Rectangle, Size},
    prelude::*,
    sync::Mutex,
};
use alloc::sync::{Arc, Weak};
use custom_debug_derive::Debug as CustomDebug;

#[derive(CustomDebug)]
pub(crate) struct Window {
    size: Size<i32>,
    drawer: Arc<Mutex<WindowDrawer>>,
    transparent_color: Option<Color>,
    #[debug(skip)]
    shadow_buffer: ShadowBuffer,
}

impl Window {
    pub(crate) fn new(size: Size<i32>) -> Arc<Mutex<Self>> {
        let screen_info = *framebuffer::info();
        #[allow(clippy::unwrap_used)]
        let window = Arc::new(Mutex::new(Self {
            size,
            drawer: Arc::new(Mutex::new(WindowDrawer {
                size,
                window: Weak::new(),
            })),
            transparent_color: None,
            shadow_buffer: ShadowBuffer::new_shadow(size, screen_info).unwrap(),
        }));
        window.lock().drawer.lock().window = Arc::downgrade(&window);
        window
    }

    pub(crate) fn drawer(&self) -> Arc<Mutex<WindowDrawer>> {
        self.drawer.clone()
    }

    pub(crate) fn set_transparent_color(&mut self, tc: Option<Color>) {
        self.transparent_color = tc;
    }

    fn colors(&'_ self) -> impl Iterator<Item = (Color, Point<i32>)> + '_ {
        self.shadow_buffer
            .area()
            .points()
            .filter_map(move |p| self.shadow_buffer.color_at(p).map(|c| (c, p)))
    }

    pub(crate) fn draw_to(&self, drawer: &mut framebuffer::Drawer, pos: Point<i32>) {
        match self.transparent_color {
            Some(tc) => {
                for (c, wp) in self.colors() {
                    if tc != c {
                        drawer.draw(pos + wp, c);
                    }
                }
            }
            None => drawer.copy(pos, &self.shadow_buffer),
        }
    }
}

#[derive(Debug)]
pub(crate) struct WindowDrawer {
    size: Size<i32>,
    window: Weak<Mutex<Window>>,
}

impl Draw for WindowDrawer {
    fn size(&self) -> Size<i32> {
        self.size
    }

    fn draw(&mut self, p: Point<i32>, c: Color) {
        if let Some(window) = self.window.upgrade() {
            let mut window = window.lock();
            window.shadow_buffer.draw(p, c);
        }
    }

    fn move_area(&mut self, offset: Point<i32>, src: Rectangle<i32>) {
        if let Some(window) = self.window.upgrade() {
            let mut window = window.lock();
            window.shadow_buffer.move_area(offset, src)
        }
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
