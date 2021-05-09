#![warn(clippy::unwrap_used)]
#![warn(clippy::expect_used)]

use core::{convert::TryFrom, fmt};

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

impl<T> TryFrom<(T, T, T)> for Color
where
    u8: TryFrom<T>,
{
    type Error = <u8 as TryFrom<T>>::Error;

    fn try_from((r, g, b): (T, T, T)) -> Result<Self, Self::Error> {
        let (r, g, b) = (u8::try_from(r)?, u8::try_from(g)?, u8::try_from(b)?);
        Ok(Self { r, g, b })
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
    pub(crate) fn new(x: T, y: T) -> Self {
        Self { x, y }
    }
}

impl<T, U> TryFrom<(U, U)> for Vector2d<T>
where
    T: TryFrom<U>,
{
    type Error = T::Error;

    fn try_from((x, y): (U, U)) -> Result<Self, Self::Error> {
        let (x, y) = (T::try_from(x)?, T::try_from(y)?);
        Ok(Self { x, y })
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
