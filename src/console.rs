use crate::{
    desktop,
    graphics::{font, frame_buffer, Color, Draw, FrameBufferDrawer, Point, Rectangle, Size},
    layer,
    prelude::*,
    sync::{mpsc, SpinMutex, SpinMutexGuard},
    window::Window,
};
use alloc::sync::Arc;
use core::{convert::TryFrom, fmt};
use x86_64::instructions::interrupts;

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::console::_print(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}

#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    use core::fmt::Write as _;

    interrupts::without_interrupts(|| {
        if let Ok(mut console) = CONSOLE.try_lock() {
            let _ = console.with_writer(|mut writer| {
                #[allow(clippy::unwrap_used)]
                writer.write_fmt(args).unwrap(); // never fail
            });
        }
    })
}

const ROWS: usize = 25;
const COLUMNS: usize = 80;

static CONSOLE: SpinMutex<Console> = SpinMutex::new(Console {
    buffer: [[0; COLUMNS]; ROWS],
    fg_color: desktop::FG_COLOR,
    bg_color: desktop::BG_COLOR,
    cursor: Point::new(0, 0),
    window: None,
});

pub(crate) struct Console {
    buffer: [[u8; COLUMNS]; ROWS],
    fg_color: Color,
    bg_color: Color,
    cursor: Point<usize>,
    window: Option<(Arc<SpinMutex<Window>>, mpsc::Sender<()>)>,
}

#[derive(Debug)]
struct RedrawArea {
    area: Option<Rectangle<usize>>,
    fill_bg: bool,
    scroll: usize,
}

impl RedrawArea {
    fn new() -> Self {
        Self {
            area: None,
            fill_bg: false,
            scroll: 0,
        }
    }

    fn all(fill_bg: bool) -> Self {
        Self {
            area: Some(Rectangle {
                pos: Point::new(0, 0),
                size: Size::new(COLUMNS, ROWS),
            }),
            fill_bg,
            scroll: 0,
        }
    }

    fn add(&mut self, p: Point<usize>) {
        match &mut self.area {
            Some(area) => *area = area.extend_to_contain(p),
            None => self.area = Some(Rectangle::new(p, Size::new(1, 1))),
        }
    }

    fn scroll(&mut self) {
        self.scroll += 1;
        if let Some(area) = self.area {
            let mut start = area.pos;
            start.y = start.y.saturating_sub(1);
            let mut end = area.end_pos();
            end.y = end.y.saturating_sub(1);
            self.area = Rectangle::from_points(start, end);
        }
    }
}

impl Console {
    fn write_str(&mut self, s: &str) -> RedrawArea {
        let mut redraw = RedrawArea::new();
        for ch in s.chars() {
            let byte = font::char_to_byte(ch);
            if byte == b'\n' {
                self.newline(&mut redraw);
                continue;
            }

            if self.cursor.x >= COLUMNS - 1 {
                self.newline(&mut redraw);
            }
            redraw.add(self.cursor);
            self.buffer[self.cursor.y][self.cursor.x] = byte;
            self.cursor.x += 1;
        }
        redraw
    }

    fn newline(&mut self, redraw: &mut RedrawArea) {
        self.cursor.x = 0;
        if self.cursor.y < ROWS - 1 {
            self.cursor.y += 1;
            return;
        }

        for (src, dst) in (1..).zip(0..(ROWS - 1)) {
            self.buffer[dst] = self.buffer[src];
        }
        self.buffer[ROWS - 1].fill(0b0);
        redraw.scroll();
    }

    fn set_window(
        &mut self,
        window: Option<(Arc<SpinMutex<Window>>, mpsc::Sender<()>)>,
    ) -> Result<()> {
        self.window = window;
        self.refresh()?;
        Ok(())
    }

    fn refresh(&mut self) -> Result<()> {
        self.with_writer(|mut writer| {
            writer.redraw(RedrawArea::all(true));
        })
    }

    fn with_writer(&'_ mut self, f: impl FnOnce(ConsoleWriter<'_, '_>)) -> Result<()> {
        assert!(!interrupts::are_enabled());

        if let Some((window, tx)) = self.window.clone() {
            let drawer = Drawer::Window(window.lock());
            let writer = ConsoleWriter {
                drawer,
                console: self,
            };
            f(writer);
            tx.send(())?;
        } else {
            let drawer = Drawer::FrameBuffer(frame_buffer::lock_drawer());
            let writer = ConsoleWriter {
                drawer,
                console: self,
            };
            f(writer);
        }
        Ok(())
    }
}

enum Drawer<'a> {
    FrameBuffer(SpinMutexGuard<'static, FrameBufferDrawer>),
    Window(SpinMutexGuard<'a, Window>),
}

impl<'a> Drawer<'a> {
    fn with_drawer<T>(&self, f: impl FnOnce(&dyn Draw) -> T) -> T {
        match self {
            Self::FrameBuffer(drawer) => f(&**drawer),
            Self::Window(drawer) => f(&**drawer),
        }
    }

    fn with_drawer_mut<T>(&mut self, f: impl FnOnce(&mut dyn Draw) -> T) -> T {
        match self {
            Self::FrameBuffer(drawer) => f(&mut **drawer),
            Self::Window(drawer) => f(&mut **drawer),
        }
    }
}

impl Draw for Drawer<'_> {
    fn size(&self) -> Size<i32> {
        self.with_drawer(|d| d.size())
    }

    fn draw(&mut self, p: Point<i32>, c: Color) {
        self.with_drawer_mut(|d| d.draw(p, c))
    }

    fn move_area(&mut self, offset: Point<i32>, src: Rectangle<i32>) {
        self.with_drawer_mut(|d| d.move_area(offset, src))
    }
}

pub(crate) struct ConsoleWriter<'d, 'c> {
    drawer: Drawer<'d>,
    console: &'c mut Console,
}

impl<'d, 'c> ConsoleWriter<'d, 'c> {
    fn to_draw_point(&self, p: Point<usize>) -> Point<i32> {
        let font_size = font::FONT_PIXEL_SIZE;
        #[allow(clippy::unwrap_used)]
        Point {
            x: i32::try_from(p.x).unwrap() * font_size.x,
            y: i32::try_from(p.y).unwrap() * font_size.y,
        }
    }

    fn to_draw_rect(&self, rect: Rectangle<usize>) -> Rectangle<i32> {
        Rectangle {
            pos: self.to_draw_point(rect.pos),
            size: self.to_draw_point(rect.size),
        }
    }

    fn redraw(&mut self, redraw: RedrawArea) {
        if redraw.scroll > 0 {
            let src = self.to_draw_rect(Rectangle {
                pos: Point::new(0, 0),
                size: Size::new(COLUMNS, ROWS),
            });
            let offset = -self.to_draw_point(Point::new(0, redraw.scroll));
            self.drawer.move_area(offset, src);
            let fill = self.to_draw_rect(Rectangle {
                pos: Point::new(0, ROWS - redraw.scroll),
                size: Size::new(COLUMNS, redraw.scroll),
            });
            self.drawer.fill_rect(fill, self.console.bg_color);
        }

        if let Some(area) = redraw.area {
            if redraw.fill_bg {
                let rect = self.to_draw_rect(area);
                self.drawer.fill_rect(rect, self.console.bg_color);
            }

            for console_y in area.y_range() {
                let x_range = area.x_range();
                let console_p = Point::new(area.x_start(), console_y);

                let bytes = &self.console.buffer[console_y][x_range];
                let draw_p = self.to_draw_point(console_p);
                self.drawer
                    .draw_byte_str(draw_p, bytes, self.console.fg_color);
            }
        }
    }
}

impl<'d, 'c> fmt::Write for ConsoleWriter<'d, 'c> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let redraw = self.console.write_str(s);
        self.redraw(redraw);
        Ok(())
    }
}

pub(crate) struct ConsoleInitParam {
    window: Arc<SpinMutex<Window>>,
    rx: mpsc::Receiver<()>,
}

pub(crate) fn start_window_mode() -> Result<ConsoleInitParam> {
    let font_size = font::FONT_PIXEL_SIZE;
    let window_size = Size::new(COLUMNS as i32 * font_size.x, ROWS as i32 * font_size.y);
    let window = Window::builder()
        .size(window_size)
        .height(layer::CONSOLE_HEIGHT)
        .build()?;
    let window = Arc::new(SpinMutex::new(window));
    let (tx, rx) = mpsc::channel(100);
    {
        interrupts::without_interrupts(|| {
            CONSOLE.lock().set_window(Some((window.clone(), tx)))?;
            Ok::<(), Error>(())
        })?;
    }
    Ok(ConsoleInitParam { window, rx })
}

pub(crate) async fn handler_task(param: ConsoleInitParam) -> Result<()> {
    let ConsoleInitParam { window, mut rx } = param;
    window.lock().flush().await?;

    while let Some(()) = rx.next().await {
        window.lock().flush().await?;
    }

    Ok(())
}
