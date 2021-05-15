#pragma once

#ifdef __cplusplus
#include <cstddef>
#include <cstdint>
extern "C" {
#else
#include <stddef.h>
#include <stdin.h>
#endif

int32_t sabios_log(int32_t level, const char *msg, size_t len);

#ifdef __cplusplus
}
#endif
