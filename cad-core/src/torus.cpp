#include "internal/shape_impl.hpp"

#include <BRepPrimAPI_MakeTorus.hxx>
#include <Precision.hxx>
#include <Standard_Failure.hxx>
#include <cmath>

namespace forma {

std::unique_ptr<Shape> make_torus(double major_radius_mm, double minor_radius_mm) noexcept {
  if (!std::isfinite(major_radius_mm) || !std::isfinite(minor_radius_mm) ||
      major_radius_mm > 10000.0 || minor_radius_mm <= Precision::Confusion() ||
      major_radius_mm - minor_radius_mm <= Precision::Confusion()) {
    return failure("INVALID_DIMENSION", "Torus requires bounded radii and a positive tube and hole");
  }
  try {
    return checked(BRepPrimAPI_MakeTorus(major_radius_mm, minor_radius_mm).Shape());
  } catch (const Standard_Failure& error) {
    return failure("OCCT_ERROR", error.GetMessageString());
  } catch (...) {
    return failure("NATIVE_ERROR", "Torus construction failed");
  }
}

} // namespace forma
