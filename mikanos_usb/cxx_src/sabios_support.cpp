#include "logger.hpp"
#include "usb/classdriver/mouse.hpp"
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

extern "C" typedef void (*ObserverType)(int8_t displacement_x, int8_t displacement_y);

extern "C" void cxx_xhci_hid_mouse_driver_set_default_observer(ObserverType observer) {
  usb::HIDMouseDriver::default_observer = observer;
}
