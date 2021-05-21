#![warn(unsafe_op_in_unsafe_fn)]
#![warn(clippy::unwrap_used)]
#![warn(clippy::expect_used)]
#![no_std]

type MouseObserverType = extern "C" fn(buttons: u8, displacement_x: i8, displacement_y: i8);
type KeyboardObserverType = extern "C" fn(modifier: u8, keycode: u8);

extern "C" {
    fn cxx_xhci_controller_new(xhc_mmio_base: u64) -> *mut xhci::Controller;
    fn cxx_xhci_controller_initialize(xhc: *mut xhci::Controller);
    fn cxx_xhci_controller_run(xhc: *mut xhci::Controller) -> i32;
    fn cxx_xhci_controller_configure_connected_ports(xhc: *mut xhci::Controller);
    fn cxx_xhci_controller_process_event(xhc: *mut xhci::Controller) -> i32;
    fn cxx_xhci_controller_has_event(xhc: *mut xhci::Controller) -> bool;
    fn cxx_xhci_hid_mouse_driver_set_default_observer(observer: MouseObserverType);
    fn cxx_xhci_hid_keyboard_driver_set_default_observer(observer: KeyboardObserverType);
    fn cxx_set_memory_pool(pool_ptr: u64, pool_size: usize);
}

pub struct CxxError(pub i32);

pub mod xhci {
    use super::*;

    // opaque type
    pub enum Controller {}

    impl Controller {
        pub unsafe fn new(xhc_mmio_base: u64) -> &'static mut Controller {
            unsafe { &mut *cxx_xhci_controller_new(xhc_mmio_base) }
        }

        pub fn init(&mut self) {
            unsafe { cxx_xhci_controller_initialize(self) }
        }

        pub fn run(&mut self) -> Result<(), CxxError> {
            let res = unsafe { cxx_xhci_controller_run(self) };
            convert_res(res)
        }

        pub fn configure_connected_ports(&mut self) {
            unsafe { cxx_xhci_controller_configure_connected_ports(self) }
        }

        pub fn process_event(&mut self) -> Result<(), CxxError> {
            let res = unsafe { cxx_xhci_controller_process_event(self) };
            convert_res(res)
        }

        pub fn has_event(&mut self) -> bool {
            unsafe { cxx_xhci_controller_has_event(self) }
        }
    }
}

// opaque type
pub enum HidMouseDriver {}

pub type HidMouseObserver = extern "C" fn(buttons: u8, displacement_x: i8, displacement_y: i8);

impl HidMouseDriver {
    pub fn set_default_observer(observer: HidMouseObserver) {
        unsafe { cxx_xhci_hid_mouse_driver_set_default_observer(observer) }
    }
}

// opaque type
pub enum HidKeyboardDriver {}

pub type HidKeyboardObserver = extern "C" fn(modifier: u8, keycode: u8);

impl HidKeyboardDriver {
    pub fn set_default_observer(observer: HidKeyboardObserver) {
        unsafe { cxx_xhci_hid_keyboard_driver_set_default_observer(observer) }
    }
}

pub unsafe fn set_memory_pool(pool_ptr: u64, pool_size: usize) {
    unsafe {
        cxx_set_memory_pool(pool_ptr, pool_size);
    }
}

#[track_caller]
fn convert_res(res: i32) -> Result<(), CxxError> {
    match res {
        0 => Ok(()),
        n => Err(CxxError(n)),
    }
}
