pub(crate) mod lapic {
    use crate::{
        acpi,
        interrupt::{self, InterruptIndex},
        prelude::*,
        sync::{mpsc, oneshot, OnceCell},
    };
    use alloc::collections::BinaryHeap;
    use core::{
        cmp,
        pin::Pin,
        sync::atomic::{AtomicU64, Ordering},
        task::{Context, Poll},
    };
    use futures_util::{select_biased, task::AtomicWaker, Future, FutureExt, Stream, StreamExt};
    use volatile::Volatile;
    use x86_64::structures::idt::InterruptStackFrame;

    const COUNT_MAX: u32 = 0xffffffff;

    fn lvt_timer() -> Volatile<&'static mut u32> {
        unsafe { Volatile::new((0xfee00320u64 as *mut u32).as_mut().unwrap()) }
    }
    fn initial_count() -> Volatile<&'static mut u32> {
        unsafe { Volatile::new((0xfee00380u64 as *mut u32).as_mut().unwrap()) }
    }
    fn current_count() -> Volatile<&'static mut u32> {
        unsafe { Volatile::new((0xfee00390u64 as *mut u32).as_mut().unwrap()) }
    }
    fn divide_config() -> Volatile<&'static mut u32> {
        unsafe { Volatile::new((0xfee003e0u64 as *mut u32).as_mut().unwrap()) }
    }

    const TIMER_FREQ: u32 = 100;

    pub(crate) fn init() {
        divide_config().write(0b1011); // divide 1:1
        lvt_timer().write(0b001 << 16); // masked, one-shot

        start();
        acpi::wait_milliseconds(100);
        let elapsed = elapsed();
        stop();

        let lapic_timer_freq = elapsed * 10;

        divide_config().write(0b1011); // divide 1:1
        lvt_timer().write((0b010 << 16) | (InterruptIndex::Timer as u32)); // not-masked, periodic
        initial_count().write(lapic_timer_freq / TIMER_FREQ);
    }

    pub(crate) fn start() {
        initial_count().write(COUNT_MAX);
    }

    pub(crate) fn elapsed() -> u32 {
        COUNT_MAX - current_count().read()
    }

    pub(crate) fn stop() {
        initial_count().write(0);
    }

    pub(crate) fn register(timeout: u64) -> Result<oneshot::Receiver<u64>> {
        let (tx, rx) = oneshot::channel();
        let timer = Timer { timeout, tx };
        TIMER_TX.get().send(timer)?;
        Ok(rx)
    }

    #[derive(Debug)]
    struct Timer {
        timeout: u64,
        tx: oneshot::Sender<u64>,
    }

    impl PartialEq for Timer {
        fn eq(&self, other: &Self) -> bool {
            self.timeout == other.timeout
        }
    }

    impl Eq for Timer {}

    impl PartialOrd for Timer {
        fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
            Some(self.cmp(other))
        }
    }

    impl Ord for Timer {
        fn cmp(&self, other: &Self) -> cmp::Ordering {
            self.timeout.cmp(&other.timeout).reverse()
        }
    }

    #[derive(Debug)]
    struct TimerManager {
        tick: u64,
        timers: BinaryHeap<Timer>,
    }

    impl TimerManager {
        fn new() -> Self {
            Self {
                tick: 0,
                timers: BinaryHeap::new(),
            }
        }

        fn register(&mut self, timer: Timer) {
            self.timers.push(timer);
            self.fire_timers();
        }

        fn tick(&mut self, count: u64) {
            self.tick += count;
            self.fire_timers();
        }

        fn fire_timers(&mut self) {
            while let Some(timer) = self.timers.peek() {
                if timer.timeout > self.tick {
                    break;
                }
                let timer = self.timers.pop().unwrap();
                timer.tx.send(timer.timeout);
            }
        }
    }

    static INTERRUPTED_COUNT: AtomicU64 = AtomicU64::new(0);
    static WAKER: AtomicWaker = AtomicWaker::new();
    static TIMER_TX: OnceCell<mpsc::Sender<Timer>> = OnceCell::uninit();

    #[derive(Debug)]
    struct InterruptStream {
        _private: (),
    }

    impl InterruptStream {
        fn new() -> Self {
            Self { _private: () }
        }
    }

    impl Stream for InterruptStream {
        type Item = u64;

        fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
            // fast path
            let count = INTERRUPTED_COUNT.swap(0, Ordering::Relaxed);
            if count > 0 {
                return Poll::Ready(Some(count));
            }

            WAKER.register(&cx.waker());
            let count = INTERRUPTED_COUNT.swap(0, Ordering::Relaxed);
            if count > 0 {
                WAKER.take();
                Poll::Ready(Some(count))
            } else {
                Poll::Pending
            }
        }
    }

    pub(crate) extern "x86-interrupt" fn interrupt_handler(_stack_frame: InterruptStackFrame) {
        INTERRUPTED_COUNT.fetch_add(1, Ordering::Relaxed);
        WAKER.wake();
        interrupt::notify_end_of_interrupt();
    }

    pub(crate) fn handler_task() -> impl Future<Output = ()> {
        // Initialize TIMER_TX before co-task starts
        let (tx, mut rx) = mpsc::channel(100);
        TIMER_TX.init_once(|| tx);

        async move {
            let mut timer_manager = TimerManager::new();
            let mut interrupts = InterruptStream::new();
            loop {
                select_biased! {
                    count = interrupts.next().fuse() => {
                        #[allow(clippy::unwrap_used)]
                        timer_manager.tick(count.unwrap());
                    },
                    timer = rx.next().fuse() => {
                        #[allow(clippy::unwrap_used)]
                        timer_manager.register(timer.unwrap());
                    }
                }
            }
        }
    }
}
