#![warn(clippy::unwrap_used)]
#![warn(clippy::expect_used)]

use core::{
    convert::TryFrom,
    fmt, iter,
    ops::{Add, Range},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Color {
    pub(crate) r: u8,
    pub(crate) g: u8,
    pub(crate) b: u8,
}

#[allow(dead_code)]
impl Color {
    pub(crate) const RED: Self = Color::new(255, 0, 0);
    pub(crate) const GREEN: Self = Color::new(0, 255, 0);
    pub(crate) const BLUE: Self = Color::new(0, 0, 255);
    pub(crate) const BLACK: Self = Color::new(0, 0, 0);
    pub(crate) const WHITE: Self = Color::new(255, 255, 255);
}

impl Color {
    pub(crate) const fn new(r: u8, g: u8, b: u8) -> Self {
        Color { r, g, b }
    }

    pub(crate) fn to_grayscale(self) -> u8 {
        #[allow(clippy::unwrap_used)] // this never panics
        u8::try_from((u16::from(self.r) + u16::from(self.g) + u16::from(self.b)) / 3).unwrap()
    }
}

impl fmt::Display for Color {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Vector2d<T> {
    pub(crate) x: T,
    pub(crate) y: T,
}

impl<T> Vector2d<T> {
    pub(crate) const fn new(x: T, y: T) -> Self {
        Self { x, y }
    }
}

impl<T> fmt::Display for Vector2d<T>
where
    T: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}, {})", self.x, self.y)
    }
}

pub(crate) type Point<T> = Vector2d<T>;
pub(crate) type Size<T> = Vector2d<T>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Rectangle<T> {
    pub(crate) pos: Point<T>,
    pub(crate) size: Size<T>,
}

impl<T> Rectangle<T> {
    pub(crate) const fn new(pos: Point<T>, size: Size<T>) -> Self {
        Self { pos, size }
    }
}

impl<T> Rectangle<T>
where
    T: Copy + Add<Output = T>,
{
    pub(crate) fn x_range(&self) -> Range<T> {
        self.pos.x..(self.pos.x + self.size.x)
    }

    pub(crate) fn y_range(&self) -> Range<T> {
        self.pos.y..(self.pos.y + self.size.y)
    }
}

impl<T> Rectangle<T>
where
    T: Copy + Add<Output = T> + PartialOrd,
{
    pub(crate) fn contains(&self, p: &Point<T>) -> bool {
        self.x_range().contains(&p.x) && self.y_range().contains(&p.y)
    }
}

impl<T> Rectangle<T>
where
    T: Copy + Add<Output = T>,
    Range<T>: Iterator<Item = T>,
{
    pub(crate) fn points(self) -> impl Iterator<Item = Point<T>> {
        self.x_range()
            .flat_map(move |x| iter::repeat(x).zip(self.y_range()))
            .map(|(x, y)| Point::new(x, y))
    }
}

pub(crate) trait Draw {
    fn area(&self) -> Rectangle<i32>;
    fn draw(&mut self, p: Point<i32>, c: Color);

    fn fill_rect(&mut self, rect: Rectangle<i32>, c: Color) {
        for p in rect.points() {
            self.draw(p, c);
        }
    }
}
static_assertions::assert_obj_safe!(Draw);
