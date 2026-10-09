#include "forma_core.hpp"

#include <cassert>
#include <cmath>

int main() {
  const double rectangle[] = {0, 0, 20, 0, 20, 10, 0, 10};
  for (int plane = 0; plane < 3; ++plane) {
    auto solid = forma::make_polygon_prism(rectangle, 8, plane, 5, 6, 7, 12);
    assert(solid && solid->ok());
    assert(std::abs(solid->volume_mm3() - 2400.0) < 0.01);
    assert(solid->face_count() == 6);
  }
  auto reversed = forma::make_polygon_prism(rectangle, 8, 0, 0, 0, 0, -12);
  assert(reversed && reversed->ok());
  assert(std::abs(reversed->volume_mm3() - 2400.0) < 0.01);
  const double triangle[] = {0, 0, 20, 0, 0, 10};
  auto triangular = forma::make_polygon_prism(triangle, 6, 0, 0, 0, 0, 5);
  assert(triangular && triangular->ok());
  assert(std::abs(triangular->volume_mm3() - 500.0) < 0.01);
  auto invalid = forma::make_polygon_prism(rectangle, 7, 0, 0, 0, 0, 5);
  assert(invalid && !invalid->ok());
}
