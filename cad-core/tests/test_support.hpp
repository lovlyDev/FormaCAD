#pragma once

#include <cstdio>
#include <exception>
#include <stdexcept>

inline void require(bool condition, const char* message) {
  if (!condition) throw std::runtime_error(message);
}

template <class Test>
int run_test(Test&& test) {
  try {
    test();
    return 0;
  } catch (const std::exception& error) {
    std::fprintf(stderr, "CAD core test failed: %s\n", error.what());
    return 1;
  }
}
