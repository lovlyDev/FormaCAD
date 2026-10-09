#include "forma_core.hpp"

#include <BRepPrimAPI_MakeSphere.hxx>
#include <Standard_Failure.hxx>

#include <cmath>

namespace forma {

std::unique_ptr<Shape> make_sphere(double radius_mm) noexcept {
  if (!std::isfinite(radius_mm) || radius_mm <= 0.0 || radius_mm > 10000.0)
    return failure("INVALID_DIMENSION", "Sphere radius must be positive and finite");
  try {
    return checked(BRepPrimAPI_MakeSphere(radius_mm).Shape());
  } catch (const Standard_Failure& error) {
    return failure("OCCT_ERROR", error.GetMessageString());
  } catch (...) {
    return failure("NATIVE_ERROR", "Sphere construction failed");
  }
}

} // namespace forma
