#include "forma_core.hpp"
#include "internal/profile_face.hpp"
#include <BRepPrimAPI_MakePrism.hxx>
#include <Standard_Failure.hxx>
#include <gp_Vec.hxx>
#include <cmath>
#include <stdexcept>

namespace forma {
std::unique_ptr<Shape> make_profile_prism(const double* coordinates, std::size_t coordinate_count,
    const std::size_t* offsets, std::size_t offset_count, int plane,
    double ox, double oy, double oz, double distance) noexcept {
  if (!std::isfinite(distance) || distance == 0 || std::abs(distance) > 10000)
    return failure("INVALID_SKETCH", "Sketch profile extrusion distance is invalid");
  try {
    const auto face = profile_face(coordinates, coordinate_count, offsets, offset_count, plane, ox, oy, oz);
    const gp_Vec normal = plane==0 ? gp_Vec(0,0,1) : plane==1 ? gp_Vec(0,-1,0) : gp_Vec(1,0,0);
    return checked(BRepPrimAPI_MakePrism(face,normal*distance).Shape());
  } catch (const std::invalid_argument& error) { return failure("INVALID_SKETCH", error.what()); }
    catch (const Standard_Failure& error) { return failure("OCCT_ERROR", error.GetMessageString()); }
    catch (...) { return failure("NATIVE_ERROR", "Sketch profile extrusion failed"); }
}
} // namespace forma