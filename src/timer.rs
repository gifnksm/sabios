pub(crate) mod lapic {
    use crate::{
        interrupt::{self, InterruptIndex},
        prelude::*,
    };
    use core::{
        pin::Pin,
        sync::atomic::{AtomicU64, Ordering},
        task::{Context, Poll},
    };
    use futures_util::{task::AtomicWaker, Stream, StreamExt};
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

    pub(crate) fn init() {
        divide_config().write(0b1011); // divide 1:1
        lvt_timer().write((0b010 << 16) | (InterruptIndex::Timer as u32)); // not-masked, periodic
        initial_count().write(0x1000000);
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

    #[derive(Debug)]
    struct TimerManager {
        tick: u64,
    }

    impl TimerManager {
        fn new() -> Self {
            Self { tick: 0 }
        }

        fn current_tick(&self) -> u64 {
            self.tick
        }

        fn tick(&mut self, count: u64) {
            self.tick = self.tick.wrapping_add(count);
        }
    }

    static INTERRUPTED_COUNT: AtomicU64 = AtomicU64::new(0);
    static WAKER: AtomicWaker = AtomicWaker::new();

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

    pub(crate) async fn handler_task() {
        let res = async {
            let mut timer_manager = TimerManager::new();
            let mut interrupts = InterruptStream::new();
            while let Some(count) = interrupts.next().await {
                timer_manager.tick(count);
                crate::println!("Timer interrupt: {}", timer_manager.current_tick());
            }
            Ok::<(), Error>(())
        }
        .await;
        if let Err(err) = res {
            panic!("error occurred during handling timer interruption: {}", err);
        }
    }
}
