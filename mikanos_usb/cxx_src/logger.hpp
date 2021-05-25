/**
 * @file logger.hpp
 *
 * カーネルロガーの実装．
 */

#pragma once

#include <cstdint>

enum LogLevel {
  kError = 3,
  kWarn = 4,
  kInfo = 6,
  kDebug = 7,
  kTrace = 8,
};

#define Log(level, ...) _Log((level), __FILE__, __LINE__, false, ##__VA_ARGS__)

/** @brief ログを指定された優先度で記録する．
 *
 * 指定された優先度がしきい値以上ならば記録する．
 * 優先度がしきい値未満ならログは捨てられる．
 *
 * @param level  ログの優先度．しきい値以上の優先度のログのみが記録される．
 * @param format  書式文字列．printk と互換．
 */
int _Log(enum LogLevel level, const char *file, uint32_t line, bool cont_line, const char *format,
         ...) __attribute__((format(printf, 5, 6)));
