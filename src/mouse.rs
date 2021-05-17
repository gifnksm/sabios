use crate::{
    desktop,
    error::ConvertErr as _,
    framebuffer,
    graphics::{Color, Draw, Point, Vector2d},
    prelude::*,
};
use conquer_once::noblock::OnceCell;
use core::{
    pin::Pin,
    task::{Context, Poll},
};
use crossbeam_queue::ArrayQueue;
use futures_util::{task::AtomicWaker, Stream, StreamExt as _};

const MOUSE_CURSOR_WIDTH: usize = 15;
const MOUSE_CURSOR_HEIGHT: usize = 24;

const MOUSE_CURSOR_SHAPE: [[u8; MOUSE_CURSOR_WIDTH]; MOUSE_CURSOR_HEIGHT] = [
    *b"@              ",
    *b"@@             ",
    *b"@.@            ",
    *b"@..@           ",
    *b"@...@          ",
    *b"@....@         ",
    *b"@.....@        ",
    *b"@......@       ",
    *b"@.......@      ",
    *b"@........@     ",
    *b"@.........@    ",
    *b"@..........@   ",
    *b"@...........@  ",
    *b"@............@ ",
    *b"@......@@@@@@@@",
    *b"@......@       ",
    *b"@....@@.@      ",
    *b"@...@ @.@      ",
    *b"@..@   @.@     ",
    *b"@.@    @.@     ",
    *b"@@      @.@    ",
    *b"@       @.@    ",
    *b"         @.@   ",
    *b"         @@@   ",
];

static MOUSE_CURSOR: spin::Mutex<MouseCursor> = spin::Mutex::new(MouseCursor {
    erase_color: desktop::BG_COLOR,
    position: Vector2d::new(300, 200),
});

struct MouseCursor {
    erase_color: Color,
    position: Point<i32>,
}

impl MouseCursor {
    fn move_relative(&mut self, displacement: Vector2d<i32>) -> Result<()> {
        self.erase()?;
        self.position += displacement;
        self.draw()?;
        Ok(())
    }

    fn erase(&mut self) -> Result<()> {
        let mut drawer = framebuffer::lock_drawer()?;
        for (dy, row) in (0..).zip(MOUSE_CURSOR_SHAPE) {
            for (dx, ch) in (0..).zip(row) {
                let p = self.position + Vector2d::new(dx, dy);
                if ch != b' ' {
                    drawer.draw(p, self.erase_color);
                }
            }
        }
        Ok(())
    }

    fn draw(&mut self) -> Result<()> {
        let mut drawer = framebuffer::lock_drawer()?;
        for (dy, row) in (0..).zip(MOUSE_CURSOR_SHAPE) {
            for (dx, ch) in (0..).zip(row) {
                let p = self.position + Vector2d::new(dx, dy);
                match ch {
                    b'@' => drawer.draw(p, Color::BLACK),
                    b'.' => drawer.draw(p, Color::WHITE),
                    _ => {}
                }
            }
        }
        Ok(())
    }
}

#[track_caller]
fn lock() -> Result<spin::MutexGuard<'static, MouseCursor>> {
    Ok(MOUSE_CURSOR
        .try_lock()
        .ok_or(ErrorKind::Deadlock("mouse::MOUSE_CURSOR"))?)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct MouseEvent {
    displacement: Vector2d<i32>,
}

static MOUSE_EVENT_QUEUE: OnceCell<ArrayQueue<MouseEvent>> = OnceCell::uninit();
static WAKER: AtomicWaker = AtomicWaker::new();

#[derive(Debug)]
struct MouseEventStream {
    _private: (),
}

impl MouseEventStream {
    fn new() -> Result<Self> {
        MOUSE_EVENT_QUEUE
            .try_init_once(|| ArrayQueue::new(100))
            .convert_err("mouse::EVENT_QUEUE")?;
        Ok(Self { _private: () })
    }
}

impl Stream for MouseEventStream {
    type Item = MouseEvent;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        #[allow(clippy::expect_used)]
        let queue = MOUSE_EVENT_QUEUE.try_get().expect("not initialized");

        // fast path
        if let Some(event) = queue.pop() {
            return Poll::Ready(Some(event));
        }

        WAKER.register(&cx.waker());
        match queue.pop() {
            Some(event) => {
                WAKER.take();
                Poll::Ready(Some(event))
            }
            None => Poll::Pending,
        }
    }
}

pub(crate) extern "C" fn observer(displacement_x: i8, displacement_y: i8) {
    let event = MouseEvent {
        displacement: Vector2d::new(i32::from(displacement_x), i32::from(displacement_y)),
    };
    let res = MOUSE_EVENT_QUEUE
        .try_get()
        .convert_err("mouse::MOUSE_EVENT_QUEUE")
        .and_then(|queue| {
            queue.push(event).map_err(|_| ErrorKind::Full)?;
            WAKER.wake();
            Ok(())
        });
    if let Err(err) = res {
        error!("failed to enqueue to the queue: {}", err);
    }
}

pub(crate) async fn handle_mouse_event() {
    let res = async {
        lock()?.draw()?;

        let mut events = MouseEventStream::new()?;
        while let Some(event) = events.next().await {
            let mut mouse_cursor = lock()?;
            mouse_cursor.move_relative(event.displacement)?;
        }
        Ok::<(), Error>(())
    }
    .await;
    if let Err(err) = res {
        panic!("error occurred during handling mouse cursor event: {}", err);
    }
}
