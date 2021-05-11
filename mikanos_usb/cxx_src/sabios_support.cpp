#include "usb/xhci/xhci.hpp"

#include <cstdint>

extern "C" usb::xhci::Controller *cxx_xhc_controller_new(uint64_t xhc_mmio_base) {
  static usb::xhci::Controller xhc{xhc_mmio_base};
  return &xhc;
}

extern "C" void cxx_xhc_controller_initialize(usb::xhci::Controller *xhc) { xhc->Initialize(); }
