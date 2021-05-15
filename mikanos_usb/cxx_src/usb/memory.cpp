#include "usb/memory.hpp"

#include <cstdint>

namespace {
template <class T> T Ceil(T value, unsigned int alignment) {
  return (value + alignment - 1) & ~static_cast<T>(alignment - 1);
}

template <class T, class U> T MaskBits(T value, U mask) {
  return value & ~static_cast<T>(mask - 1);
}
} // namespace

namespace usb {
size_t memory_pool_size = 0;
uintptr_t pool_base_ptr = 0;
uintptr_t alloc_ptr = 0;

void SetMemoryPool(uintptr_t pool_ptr, size_t pool_size) {
  pool_base_ptr = alloc_ptr = pool_ptr;
  memory_pool_size = pool_size;
}

void *AllocMem(size_t size, unsigned int alignment, unsigned int boundary) {
  if (alignment > 0) {
    alloc_ptr = Ceil(alloc_ptr, alignment);
  }
  if (boundary > 0) {
    auto next_boundary = Ceil(alloc_ptr, boundary);
    if (next_boundary < alloc_ptr + size) {
      alloc_ptr = next_boundary;
    }
  }

  if (pool_base_ptr + memory_pool_size < alloc_ptr + size) {
    return nullptr;
  }

  auto p = alloc_ptr;
  alloc_ptr += size;
  return reinterpret_cast<void *>(p);
}

void FreeMem(void *p) {}
} // namespace usb
