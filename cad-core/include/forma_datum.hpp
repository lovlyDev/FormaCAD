#pragma once

#include <array>
#include <cstddef>

class TopoDS_Shape;

namespace forma::datum {

using Coordinates = std::array<double, 3>;

// Finite reference geometry for inspection and display, without solid mass.
// On invalid input or an OCCT failure, the returned shape is null.
TopoDS_Shape plane(const Coordinates& origin_mm, const Coordinates& normal,
                   double side_mm) noexcept;
TopoDS_Shape axis(const Coordinates& origin_mm, const Coordinates& direction,
                  double length_mm) noexcept;
TopoDS_Shape point(const Coordinates& origin_mm) noexcept;

} // namespace forma::datum

namespace forma {

// 0: plane, 1: axis, 2: point. Data is origin, direction, span for
// plane and axis, or only the origin for point.
bool validate_datum(int kind, const double* data, std::size_t count) noexcept;

} // namespace forma
