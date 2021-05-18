use crate::{
    framebuffer,
    graphics::{Color, Draw, Point, Size},
    shadow_buffer::ShadowBuffer,
    sync::mutex::Mutex,
};
use alloc::{
    sync::{Arc, Weak},
    vec,
    vec::Vec,
};
use core::convert::TryFrom;
use custom_debug_derive::Debug as CustomDebug;

#[derive(CustomDebug)]
pub(crate) struct Window {
    size: Size<i32>,
    data: Vec<Vec<Color>>,
    drawer: Arc<Mutex<WindowDrawer>>,
    transparent_color: Option<Color>,
    #[debug(skip)]
    shadow_buffer: ShadowBuffer,
}

impl Window {
    pub(crate) fn new(size: Size<i32>) -> Arc<Mutex<Self>> {
        let window = Arc::new(Mutex::new(Self {
            size,
            data: vec![vec![Color::BLACK; size.x as usize]; size.y as usize],
            drawer: Arc::new(Mutex::new(WindowDrawer {
                size,
                window: Weak::new(),
            })),
            transparent_color: None,
            shadow_buffer: ShadowBuffer::new(size),
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
        (0..)
            .zip(&self.data)
            .flat_map(|(y, row)| (0..).zip(row).map(move |(x, c)| (*c, Point::new(x, y))))
    }

    // fn get(&self, at: Point<i32>) -> Option<Color> {
    //     let x = usize::try_from(at.x).ok()?;
    //     let y = usize::try_from(at.y).ok()?;
    //     Some(*self.data.get(y)?.get(x)?)
    // }

    fn set(&mut self, at: Point<i32>, c: Color) {
        Some(()).and_then(|()| -> Option<()> {
            let x = usize::try_from(at.x).ok()?;
            let y = usize::try_from(at.y).ok()?;
            let r = self.data.get_mut(y)?.get_mut(x)?;
            *r = c;
            self.shadow_buffer.draw(at, c);
            Some(())
        });
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
            None => {
                drawer.copy(pos, &self.shadow_buffer);
                // for (c, wp) in self.colors() {
                //     drawer.draw(pos + wp, c)
                // }
            }
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
            window.set(p, c);
        }
    }
}
