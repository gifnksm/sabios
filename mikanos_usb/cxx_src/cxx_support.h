#pragma once

#ifdef __cplusplus
#include <cstdint>
extern "C" {
#else
#include <stdin.h>
#endif

int sabios_log(int level, const char *log);
void sabios_wait_1ms(void);

#ifdef __cplusplus
}
#endif
