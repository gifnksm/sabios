use crate::{
    fat,
    fmt::ByteString,
    framed_window::{FramedWindow, FramedWindowEvent},
    graphics::{font, Color, Draw, Offset, Point, Rectangle, Size},
    pci,
    prelude::*,
    timer,
};
use alloc::{collections::VecDeque, string::String, vec::Vec};
use core::{
    fmt::{self, Write as _},
    mem,
};
use futures_util::select_biased;

const FOREGROUND: Color = Color::WHITE;
const BACKGROUND: Color = Color::BLACK;
const BORDER_DARK: Color = Color::from_code(0x848484);
const BORDER_LIGHT: Color = Color::from_code(0xc6c6c6);

const PADDING_TOP: i32 = 4;
const PADDING_BOTTOM: i32 = 4;
const PADDING_LEFT: i32 = 4;
const PADDING_RIGHT: i32 = 4;
const PADDING_POS: Point<i32> = Point::new(PADDING_LEFT, PADDING_TOP);
const PADDING_SIZE: Size<i32> =
    Size::new(PADDING_LEFT + PADDING_RIGHT, PADDING_TOP + PADDING_BOTTOM);
const HISTORY_LEN: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Direction {
    Older,
    Newer,
}

#[derive(Debug)]
pub(crate) struct Terminal {
    text_size: Size<i32>,
    cursor: Point<i32>,
    cursor_visible: bool,
    line_buf: String,
    history: VecDeque<String>,
    history_index: Option<usize>,
    window: FramedWindow,
}

impl Terminal {
    pub(crate) fn new(title: String, pos: Point<i32>, text_size: Size<i32>) -> Result<Self> {
        let font_size = font::FONT_PIXEL_SIZE;
        let window = FramedWindow::builder(title)
            .pos(pos)
            .size(text_size * font_size + PADDING_SIZE)
            .build()?;
        Ok(Self {
            text_size,
            cursor: Point::new(0, 0),
            cursor_visible: false,
            line_buf: String::new(),
            history: VecDeque::with_capacity(HISTORY_LEN),
            history_index: None,
            window,
        })
    }

    fn draw_terminal(&mut self) {
        let area = self.window.area();
        self.window
            .draw_box(area, BACKGROUND, BORDER_DARK, BORDER_LIGHT)
    }

    fn insert_pos(&self) -> Point<i32> {
        let font_size = font::FONT_PIXEL_SIZE;
        font_size * self.cursor + PADDING_POS
    }

    fn draw_cursor(&mut self, visible: bool) {
        let font_size = font::FONT_PIXEL_SIZE;
        let color = if visible { FOREGROUND } else { BACKGROUND };
        let pos = self.insert_pos();
        self.window
            .fill_rect(Rectangle::new(pos, font_size - Size::new(1, 1)), color);
    }

    fn scroll1(&mut self) {
        let font_size = font::FONT_PIXEL_SIZE;
        self.window.move_area(
            Offset::new(0, -1) * font_size,
            Rectangle::new(
                Point::new(0, 1) * font_size + PADDING_POS,
                (self.text_size - Size::new(0, 1)) * font_size,
            ),
        );
        self.window.fill_rect(
            Rectangle::new(
                Offset::new(0, self.text_size.y - 1) * font_size + PADDING_POS,
                Size::new(self.text_size.x, 1) * font_size,
            ),
            BACKGROUND,
        );
    }

    fn newline(&mut self) {
        if self.cursor.y + 1 >= self.text_size.y {
            self.scroll1();
        } else {
            self.cursor.y += 1;
        }
        self.cursor.x = 0;
    }

    fn print_char(&mut self, ch: char) {
        self.draw_cursor(false);
        match ch {
            '\0' => {}
            '\n' => self.newline(),
            ch => {
                self.window.draw_char(self.insert_pos(), ch, FOREGROUND);
                if self.cursor.x + 1 >= self.text_size.x {
                    self.newline();
                } else {
                    self.cursor.x += 1;
                }
            }
        }
    }

    fn print_str(&mut self, s: &str) {
        for ch in s.chars() {
            self.print_char(ch);
        }
    }

    fn print_prompt(&mut self) {
        self.print_str("> ");
    }

    fn delete_backward(&mut self) {
        let font_size = font::FONT_PIXEL_SIZE;
        if self.cursor.y > 0 && self.cursor.x == 0 {
            self.cursor.x = self.text_size.x - 1;
            self.cursor.y -= 1;
        } else if self.cursor.x > 0 {
            self.cursor.x -= 1;
        } else {
            assert_eq!(self.cursor, Point::new(0, 0));
        }
        self.window
            .fill_rect(Rectangle::new(self.insert_pos(), font_size), BACKGROUND);
    }

    fn execute_line(&mut self) {
        // replace line_buf temporary to avoid borrow checker errors
        let line_buf = mem::take(&mut self.line_buf);
        let command_line = line_buf.trim().split_whitespace().collect::<Vec<_>>();
        if command_line.is_empty() {
            return;
        }
        match command_line[0] {
            "echo" => {
                for (i, arg) in command_line[1..].iter().enumerate() {
                    if i > 0 {
                        self.print_char(' ');
                    }
                    self.print_str(arg);
                }
                self.print_char('\n');
            }
            "clear" => {
                let font_size = font::FONT_PIXEL_SIZE;
                self.window.fill_rect(
                    Rectangle::new(PADDING_POS, font_size * self.text_size),
                    BACKGROUND,
                );
                self.cursor = Point::new(0, 0);
            }
            "lspci" => match pci::scan_all_bus() {
                Ok(devices) => {
                    for dev in devices {
                        let _ = writeln!(self, "{}", dev);
                    }
                }
                Err(err) => {
                    let _ = writeln!(self, "lspci: failed to scan PCI devices: {}", err);
                }
            },
            "ls" => {
                let fs = fat::lock();
                for entry in fs.root_dir().entries() {
                    let entry = match entry {
                        Ok(entry) => entry,
                        Err(_) => {
                            let _ = writeln!(self, "failed to read directory");
                            break;
                        }
                    };
                    let basename = entry.basename();
                    let extension = entry.extension();
                    if extension.is_empty() {
                        let _ = writeln!(self, "{}", ByteString(basename));
                    } else {
                        let _ =
                            writeln!(self, "{}.{}", ByteString(basename), ByteString(extension));
                    }
                }
            }
            command => {
                let _ = writeln!(self, "no such command: {}", command);
            }
        }
        self.line_buf = line_buf;
    }

    fn push_history(&mut self) {
        while self.history.len() > HISTORY_LEN - 1 {
            self.history.pop_back();
        }
        self.history.push_front(mem::take(&mut self.line_buf));
        self.history_index = None;
    }

    fn history_move(&mut self, direction: Direction) {
        self.history_index = match direction {
            Direction::Newer => match self.history_index {
                None => return,
                Some(i) if i == 0 => None,
                Some(i) => Some(i - 1),
            },
            Direction::Older => match self.history_index {
                None => Some(0),
                Some(i) if i + 1 < self.history.len() => Some(i + 1),
                Some(_) => return,
            },
        };

        while self.line_buf.pop().is_some() {
            self.delete_backward();
        }
        self.cursor.x = 0;
        self.print_prompt();
        if let Some(history_index) = self.history_index {
            let line = self.history[history_index].clone();
            self.print_str(&line);
            self.line_buf = line;
        } else {
            self.line_buf.clear();
        }
    }

    fn handle_event(&mut self, event: FramedWindowEvent) {
        match event {
            FramedWindowEvent::Keyboard(event) => {
                self.draw_cursor(false);
                match event.ascii {
                    '\0' if event.keycode == 0x51 => {
                        // down arrow
                        self.history_move(Direction::Newer);
                    }
                    '\0' if event.keycode == 0x52 => {
                        // up arrow
                        self.history_move(Direction::Older);
                    }
                    '\0' => {}
                    '\n' => {
                        self.newline();
                        self.execute_line();
                        if !self.line_buf.is_empty()
                            && !self.line_buf.starts_with(char::is_whitespace)
                        {
                            self.push_history();
                        }
                        self.print_prompt();
                    }
                    '\x08' => {
                        if self.line_buf.pop().is_some() {
                            self.delete_backward();
                        }
                    }
                    ch => {
                        self.line_buf.push(ch);
                        self.print_char(ch);
                    }
                }
                self.draw_cursor(true);
            }
        }
    }

    fn handle_timeout(&mut self) {
        self.cursor_visible = !self.cursor_visible;
        self.draw_cursor(self.cursor_visible);
    }

    pub(crate) async fn run(mut self) -> Result<()> {
        self.draw_terminal();
        self.print_prompt();
        self.window.flush().await?;

        let mut interval = timer::lapic::interval(0, 50)?;
        loop {
            select_biased! {
                event = self.window.recv_event().fuse() => {
                    let event = match event {
                        Some(event) => event?,
                        None => return Ok(()),
                    };
                    self.handle_event(event);
                }
                timeout = interval.next().fuse() => {
                    let _timeout = match timeout {
                        Some(event) => event?,
                        _ => return Ok(()),
                    };
                    self.handle_timeout();
                }
            }
            self.window.flush().await?;
        }
    }
}

impl fmt::Write for Terminal {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.print_str(s);
        Ok(())
    }
}
