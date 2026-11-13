#pragma once
#include <memory>
namespace forma {
class Shape;
// Both finite-volume halves must exist. Positive is dot(point-origin, normal)>0.
std::unique_ptr<Shape> split_by_plane(const Shape& source,
    double x, double y, double z, double nx, double ny, double nz, bool positive) noexcept;
}
