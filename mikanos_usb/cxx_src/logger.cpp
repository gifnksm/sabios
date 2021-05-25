#include "logger.hpp"

#include "cxx_support.h"

#include <cstdint>
#include <cstdio>
#include <cstring>

int _Log(enum LogLevel level, const char *file, uint32_t line, bool cont_line, const char *format,
         ...) {
  char buf[1024];
  va_list ap;

  va_start(ap, format);
  int res = vsnprintf(buf, sizeof(buf) - 1, format, ap);
  va_end(ap);

  sabios_log(level, file, strlen(file), line, buf, strlen(buf), cont_line);

  return res;
}
