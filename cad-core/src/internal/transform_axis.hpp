#pragma once
#include <cmath>

namespace forma {
inline bool valid_transform_axis(double ox, double oy, double oz, double dx, double dy, double dz) noexcept {
  return std::isfinite(ox) && std::isfinite(oy) && std::isfinite(oz) &&
      std::abs(ox) <= 10000 && std::abs(oy) <= 10000 && std::abs(oz) <= 10000 &&
      std::isfinite(dx) && std::isfinite(dy) && std::isfinite(dz) &&
      std::abs(dx) <= 1e6 && std::abs(dy) <= 1e6 && std::abs(dz) <= 1e6 &&
      dx * dx + dy * dy + dz * dz >= 1e-18;
}
} // namespace forma
