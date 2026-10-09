#include "internal/shape_impl.hpp"

#include <BRepBndLib.hxx>
#include <Bnd_Box.hxx>

#include <cmath>

namespace forma {

bool Shape::update_bounds() {
  Bnd_Box box;
  BRepBndLib::AddOptimal(impl_->shape, box, false, false);
  if (box.IsVoid() || box.IsOpen()) return false;
  double x_min, y_min, z_min, x_max, y_max, z_max;
  box.Get(x_min, y_min, z_min, x_max, y_max, z_max);
  const double values[3] = {x_max - x_min, y_max - y_min, z_max - z_min};
  for (int axis = 0; axis < 3; ++axis) {
    if (!std::isfinite(values[axis]) || values[axis] <= 0.0) return false;
    impl_->extents[axis] = values[axis];
  }
  return true;
}

double Shape::extent_x_mm() const noexcept { return ok() ? impl_->extents[0] : 0.0; }
double Shape::extent_y_mm() const noexcept { return ok() ? impl_->extents[1] : 0.0; }
double Shape::extent_z_mm() const noexcept { return ok() ? impl_->extents[2] : 0.0; }

} // namespace forma
