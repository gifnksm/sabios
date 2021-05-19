use crate::{
    buffer_drawer::ShadowBuffer,
    framebuffer,
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
