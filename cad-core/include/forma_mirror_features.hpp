#pragma once
#include "forma_core.hpp"
namespace forma {
std::unique_ptr<Shape> mirror_features(const Shape& after, const Shape& before, bool has_base,
    double ox, double oy, double oz, double nx, double ny, double nz) noexcept;
}
