#include "forma_core.hpp"

#include <BRepPrimAPI_MakeCone.hxx>
#include <Standard_Failure.hxx>

#include <cmath>

namespace forma {

std::unique_ptr<Shape> make_cone(double bottom_radius_mm, double top_radius_mm,
                                 double height_mm) noexcept {
  if (!std::isfinite(bottom_radius_mm) || bottom_radius_mm <= 0.0 ||
      bottom_radius_mm > 10000.0 || !std::isfinite(top_radius_mm) ||
      top_radius_mm < 0.0 || top_radius_mm > 10000.0 ||
      !std::isfinite(height_mm) || height_mm <= 0.0 || height_mm > 10000.0 ||
      bottom_radius_mm == top_radius_mm)
    return failure("INVALID_DIMENSION", "Cone dimensions are invalid");
  try {
    return checked(BRepPrimAPI_MakeCone(bottom_radius_mm, top_radius_mm, height_mm).Shape());
  } catch (const Standard_Failure& error) {
    return failure("OCCT_ERROR", error.GetMessageString());
  } catch (...) {
    return failure("NATIVE_ERROR", "Cone construction failed");
  }
}

} // namespace forma
