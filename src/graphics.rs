use core::{
    convert::TryFrom,
    fmt, iter,
    ops::{Add, AddAssign, Range, Sub},
};
use num_traits::One;

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

impl<T, U> Add<Vector2d<U>> for Vector2d<T>
where
    T: Add<U>,
{
    type Output = Vector2d<T::Output>;

    fn add(self, rhs: Vector2d<U>) -> Self::Output {
        Vector2d {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        }
    }
}

impl<T, U> AddAssign<Vector2d<U>> for Vector2d<T>
where
    T: AddAssign<U>,
{
    fn add_assign(&mut self, rhs: Vector2d<U>) {
        self.x += rhs.x;
        self.y += rhs.y;
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

// impl<T> Rectangle<T>
// where
//     T: Copy + Ord + Sub<Output = T>,
// {
//     pub(crate) fn from_points(p0: Point<T>, p1: Point<T>) -> Self {
//         let x_start = T::min(p0.x, p1.x);
//         let y_start = T::min(p0.y, p1.y);
//         let x_end = T::max(p0.x, p1.x);
//         let y_end = T::max(p0.y, p1.y);
//         Rectangle {
//             pos: Point::new(x_start, y_start),
//             size: Size::new(x_end - x_start, y_end - y_start),
//         }
//     }
// }

impl<T> Rectangle<T>
where
    T: Copy + Add<Output = T>,
{
    pub(crate) fn x_start(&self) -> T {
        self.pos.x
    }

    pub(crate) fn y_start(&self) -> T {
        self.pos.y
    }

    pub(crate) fn x_end(&self) -> T {
        self.pos.x + self.size.x
    }

    pub(crate) fn y_end(&self) -> T {
        self.pos.y + self.size.y
    }

    pub(crate) fn x_range(&self) -> Range<T> {
        self.x_start()..self.x_end()
    }

    pub(crate) fn y_range(&self) -> Range<T> {
        self.y_start()..self.y_end()
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
    T: Copy + Add<Output = T> + Sub<Output = T> + Ord + One,
{
    pub(crate) fn extend_to_contain(&self, p: Point<T>) -> Rectangle<T> {
        let x_start = T::min(p.x, self.x_start());
        let y_start = T::min(p.y, self.y_start());
        let x_end = T::max(p.x + One::one(), self.x_end());
        let y_end = T::max(p.y + One::one(), self.y_end());
        Rectangle {
            pos: Point::new(x_start, y_start),
            size: Size::new(x_end - x_start, y_end - y_start),
        }
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

    fn size(&self) -> Size<i32> {
        self.area().size
    }

    fn fill_rect(&mut self, rect: Rectangle<i32>, c: Color) {
        for p in rect.points() {
            self.draw(p, c);
        }
    }

    fn draw_rect(&mut self, rect: Rectangle<i32>, c: Color) {
        if rect.size.x == 0 || rect.size.y == 0 {
            return;
        }

        for x in rect.x_range() {
            self.draw(Point::new(x, rect.y_start()), c);
            self.draw(Point::new(x, rect.y_end() - 1), c);
        }
        for y in rect.y_range() {
            self.draw(Point::new(rect.x_start(), y), c);
            self.draw(Point::new(rect.x_end() - 1, y), c);
        }
    }
}
static_assertions::assert_obj_safe!(Draw);
