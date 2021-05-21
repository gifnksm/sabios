#include "logger.hpp"
#include "usb/classdriver/keyboard.hpp"
#include "usb/classdriver/mouse.hpp"
#include "usb/memory.hpp"
#include "usb/xhci/xhci.hpp"

#include <cstdint>

extern "C" usb::xhci::Controller *cxx_xhci_controller_new(uint64_t xhc_mmio_base) {
  static usb::xhci::Controller xhc{xhc_mmio_base};
  return &xhc;
}

extern "C" void cxx_xhci_controller_initialize(usb::xhci::Controller *xhc) { xhc->Initialize(); }

extern "C" int32_t cxx_xhci_controller_run(usb::xhci::Controller *xhc) {
  auto err = xhc->Run();
  return err.Cause();
}

extern "C" void cxx_xhci_controller_configure_connected_ports(usb::xhci::Controller *xhc) {
  for (int i = 1; i <= xhc->MaxPorts(); i++) {
    auto port = xhc->PortAt(i);
    Log(kDebug, "Port %d: IsConnected=%d\n", i, port.IsConnected());

    if (port.IsConnected()) {
      if (auto err = ConfigurePort(*xhc, port)) {
        Log(kError, "failed to configure port: %s at %s:%d\n", err.Name(), err.File(), err.Line());
        continue;
      }
    }
  }
}

extern "C" int32_t cxx_xhci_controller_process_event(usb::xhci::Controller *xhc) {
  auto err = ProcessEvent(*xhc);
  return err.Cause();
}

extern "C" bool cxx_xhci_controller_has_event(usb::xhci::Controller *xhc) {
  return xhc->PrimaryEventRing()->HasFront();
}

extern "C" typedef void (*MouseObserverType)(uint8_t buttons, int8_t displacement_x,
                                             int8_t displacement_y);

extern "C" void cxx_xhci_hid_mouse_driver_set_default_observer(MouseObserverType observer) {
  usb::HIDMouseDriver::default_observer = observer;
}

extern "C" typedef void (*KeyboardObserverType)(uint8_t modifier, uint8_t keycode);

extern "C" void cxx_xhci_hid_keyboard_driver_set_default_observer(KeyboardObserverType observer) {
  usb::HIDKeyboardDriver::default_observer = observer;
}

extern "C" void cxx_set_memory_pool(uintptr_t pool_ptr, size_t pool_size) {
  usb::SetMemoryPool(pool_ptr, pool_size);
}
