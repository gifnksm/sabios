use crate::graphics::{Color, Draw, Point, Rectangle, Size};
use alloc::{
    sync::{Arc, Weak},
    vec,
    vec::Vec,
};
use core::convert::TryFrom;

#[derive(Debug)]
pub(crate) struct Window {
    area: Rectangle<i32>,
    data: Vec<Vec<Color>>,
    drawer: Arc<spin::Mutex<WindowDrawer>>,
    transparent_color: Option<Color>,
}

impl Window {
    pub(crate) fn new(size: Size<i32>) -> Arc<spin::Mutex<Self>> {
        let area = Rectangle::new(Point::new(0, 0), size);
        let window = Arc::new(spin::Mutex::new(Self {
            area,
            data: vec![vec![Color::BLACK; size.x as usize]; size.y as usize],
            drawer: Arc::new(spin::Mutex::new(WindowDrawer {
                area,
                window: Weak::new(),
            })),
            transparent_color: None,
        }));
        window.try_lock().unwrap().drawer.try_lock().unwrap().window = Arc::downgrade(&window);
        window
    }

    pub(crate) fn drawer(&self) -> Arc<spin::Mutex<WindowDrawer>> {
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
            Some(())
        });
    }

    pub(crate) fn draw_to(&self, drawer: &mut dyn Draw, pos: Point<i32>) {
        match self.transparent_color {
            Some(tc) => {
                for (c, wp) in self.colors() {
                    if tc != c {
                        drawer.draw(pos + wp, c);
                    }
                }
            }
            None => {
                for (c, wp) in self.colors() {
                    drawer.draw(pos + wp, c)
                }
            }
        }
    }
}

#[derive(Debug)]
pub(crate) struct WindowDrawer {
    area: Rectangle<i32>,
    window: Weak<spin::Mutex<Window>>,
}

impl Draw for WindowDrawer {
    fn area(&self) -> Rectangle<i32> {
        self.area
    }

    fn draw(&mut self, p: Point<i32>, c: Color) {
        if let Some(window) = self.window.upgrade() {
            #[allow(clippy::unwrap_used)]
            window.try_lock().unwrap().set(p, c);
        }
    }
}
