use core::{
    fmt, iter,
    ops::{Add, AddAssign, BitAnd, Neg, Range, Sub, SubAssign},
};
use num_traits::One;

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

impl<T> Vector2d<T>
where
    T: Copy + Ord,
{
    pub(crate) fn elem_min(self, other: Self) -> Self {
        Self {
            x: T::min(self.x, other.x),
            y: T::min(self.y, other.y),
        }
    }

    pub(crate) fn elem_max(self, other: Self) -> Self {
        Self {
            x: T::max(self.x, other.x),
            y: T::max(self.y, other.y),
        }
    }
}

impl<T> Vector2d<T>
where
    T: Copy + Ord + Add<Output = T> + Sub<Output = T> + One,
{
    pub(crate) fn clamp(&self, rect: Rectangle<T>) -> Option<Self> {
        Some(Self {
            x: T::clamp(self.x, rect.x_start(), rect.x_max()?),
            y: T::clamp(self.y, rect.y_start(), rect.y_max()?),
        })
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

impl<T, U> Neg for Vector2d<T>
where
    T: Neg<Output = U>,
{
    type Output = Vector2d<U>;

    fn neg(self) -> Self::Output {
        Vector2d {
            x: -self.x,
            y: -self.y,
        }
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

impl<T, U> Sub<Vector2d<U>> for Vector2d<T>
where
    T: Sub<U>,
{
    type Output = Vector2d<T::Output>;

    fn sub(self, rhs: Vector2d<U>) -> Self::Output {
        Vector2d {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
        }
    }
}

impl<T, U> SubAssign<Vector2d<U>> for Vector2d<T>
where
    T: SubAssign<U>,
{
    fn sub_assign(&mut self, rhs: Vector2d<U>) {
        self.x -= rhs.x;
        self.y -= rhs.y;
    }
}

pub(crate) type Point<T> = Vector2d<T>;
pub(crate) type Size<T> = Vector2d<T>;
pub(crate) type Offset<T> = Vector2d<T>;

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
    T: Copy + Ord + Sub<Output = T>,
{
    pub(crate) fn from_points(start: Point<T>, end: Point<T>) -> Option<Self> {
        let size = Size::new(
            (end.x > start.x).then(|| end.x - start.x)?,
            (end.y > start.y).then(|| end.y - start.y)?,
        );
        Some(Rectangle { pos: start, size })
    }
}

impl<T> Rectangle<T>
where
    T: Copy + Add<T>,
{
    pub(crate) fn end_pos(&self) -> Point<T::Output> {
        self.pos + self.size
    }
}

impl<T> fmt::Display for Rectangle<T>
where
    T: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}:{})", self.pos, self.size)
    }
}

impl<T> BitAnd<Rectangle<T>> for Rectangle<T>
where
    T: Copy + Ord + Add<Output = T> + Sub<Output = T>,
{
    type Output = Option<Self>;

    fn bitand(self, rhs: Rectangle<T>) -> Self::Output {
        let start = Point::<T>::elem_max(self.pos, rhs.pos);
        let end = Point::<T>::elem_min(self.end_pos(), rhs.end_pos());
        Rectangle::from_points(start, end)
    }
}

impl<T, U> Add<Vector2d<U>> for Rectangle<T>
where
    T: Add<U, Output = T>,
{
    type Output = Self;

    fn add(self, rhs: Vector2d<U>) -> Self::Output {
        Self {
            pos: self.pos + rhs,
            size: self.size,
        }
    }
}

impl<T, U> AddAssign<Vector2d<U>> for Rectangle<T>
where
    T: AddAssign<U>,
{
    fn add_assign(&mut self, rhs: Vector2d<U>) {
        self.pos += rhs;
    }
}

impl<T, U> Sub<Vector2d<U>> for Rectangle<T>
where
    T: Sub<U, Output = T>,
{
    type Output = Self;

    fn sub(self, rhs: Vector2d<U>) -> Self::Output {
        Self {
            pos: self.pos - rhs,
            size: self.size,
        }
    }
}

impl<T, U> SubAssign<Vector2d<U>> for Rectangle<T>
where
    T: SubAssign<U>,
{
    fn sub_assign(&mut self, rhs: Vector2d<U>) {
        self.pos -= rhs;
    }
}

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
    T: Copy + Ord + Add<Output = T> + Sub<Output = T> + One,
{
    pub(crate) fn x_max(&self) -> Option<T> {
        (self.x_end() > T::one()).then(|| self.x_end() - T::one())
    }

    pub(crate) fn y_max(&self) -> Option<T> {
        (self.y_end() > T::one()).then(|| self.y_end() - T::one())
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
        let x_end = T::max(p.x + T::one(), self.x_end());
        let y_end = T::max(p.y + T::one(), self.y_end());
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
