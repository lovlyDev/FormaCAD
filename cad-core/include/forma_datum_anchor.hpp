#pragma once

#include <cstddef>
#include <string>

namespace forma {
class Shape;

// Exact origin and unit direction in CAD coordinates. Face anchors use the
// area centroid and outward normal. Edge anchors require a straight edge.
bool datum_anchor(const Shape& solid, int kind, const std::string& owner,
                  const std::string& role, const std::string& occurrence,
                  double* frame, std::size_t count) noexcept;
}
