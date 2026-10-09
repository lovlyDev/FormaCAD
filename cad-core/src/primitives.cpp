#include "forma_core.hpp"

#include <BRepPrimAPI_MakeBox.hxx>
#include <BRepPrimAPI_MakeCylinder.hxx>
#include <Standard_Failure.hxx>
#include <gp_Pnt.hxx>

#include <cmath>

namespace forma {

static bool valid_size(double value) noexcept {
  return std::isfinite(value) && value > 0.0 && value <= 10000.0;
}

std::unique_ptr<Shape> make_box(double width, double depth, double height) noexcept {
  if (!valid_size(width) || !valid_size(depth) || !valid_size(height))
    return failure("INVALID_DIMENSION", "Box dimensions must be positive and finite");
  try {
    return checked(BRepPrimAPI_MakeBox(gp_Pnt(-width / 2, -depth / 2, 0), width, depth, height).Shape());
  } catch (const Standard_Failure& error) {
    return failure("OCCT_ERROR", error.GetMessageString());
  } catch (...) {
    return failure("NATIVE_ERROR", "Box construction failed");
  }
}

std::unique_ptr<Shape> make_cylinder(double radius, double height) noexcept {
  if (!valid_size(radius) || !valid_size(height))
    return failure("INVALID_DIMENSION", "Cylinder dimensions must be positive and finite");
  try {
    return checked(BRepPrimAPI_MakeCylinder(radius, height).Shape());
  } catch (const Standard_Failure& error) {
    return failure("OCCT_ERROR", error.GetMessageString());
  } catch (...) {
    return failure("NATIVE_ERROR", "Cylinder construction failed");
  }
}

} // namespace forma
