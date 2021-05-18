pub(crate) mod lapic {
    use volatile::Volatile;

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
        lvt_timer().write((0b01 << 16) | 32); // masked, one-shot
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
}
