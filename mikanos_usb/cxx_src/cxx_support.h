#pragma once

#ifdef __cplusplus
#include <cstddef>
#include <cstdint>
extern "C" {
#else
#include <stddef.h>
#include <stdin.h>
#endif

int32_t sabios_log(int32_t level, const char *file, size_t file_len, uint32_t line, const char *msg,
                   size_t msg_len, bool cont_line);

#ifdef __cplusplus
}
#endif
