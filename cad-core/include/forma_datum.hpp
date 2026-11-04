#pragma once

#include <array>

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
