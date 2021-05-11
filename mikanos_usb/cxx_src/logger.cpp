#include "logger.hpp"

#include "cxx_support.h"

#include <cstdio>

int Log(enum LogLevel level, const char *format, ...) {
  char buf[1024];
  va_list ap;

  va_start(ap, format);
  int res = vsnprintf(buf, sizeof(buf) - 1, format, ap);
  va_end(ap);

  sabios_log(level, buf);

  return res;
}
