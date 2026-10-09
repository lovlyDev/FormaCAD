#include "forma_core.hpp"

#include <BRepBuilderAPI_MakeFace.hxx>
#include <BRepBuilderAPI_MakePolygon.hxx>
#include <BRepPrimAPI_MakePrism.hxx>
#include <Standard_Failure.hxx>
#include <gp_Pnt.hxx>
#include <gp_Vec.hxx>

#include <cmath>

namespace forma {

std::unique_ptr<Shape> make_polygon_prism(const double* outline_xy, std::size_t coordinate_count, int plane,
                                          double origin_x_mm, double origin_y_mm,
                                          double origin_z_mm, double distance_mm) noexcept {
  if (outline_xy == nullptr || coordinate_count < 6 || coordinate_count > 64 || coordinate_count % 2 != 0 ||
      plane < 0 || plane > 2 || !std::isfinite(distance_mm) || distance_mm == 0.0 ||
      std::abs(distance_mm) > 10000.0 || !std::isfinite(origin_x_mm) ||
      !std::isfinite(origin_y_mm) || !std::isfinite(origin_z_mm))
    return failure("INVALID_SKETCH", "Polygon workplane or extrusion distance is invalid");
  try {
    BRepBuilderAPI_MakePolygon polygon;
    for (std::size_t index = 0; index < coordinate_count; index += 2) {
      const double u = outline_xy[index];
      const double v = outline_xy[index + 1];
      if (!std::isfinite(u) || !std::isfinite(v) || std::abs(u) > 10000.0 || std::abs(v) > 10000.0)
        return failure("INVALID_SKETCH", "Polygon coordinates must be finite and bounded");
      const gp_Pnt point = plane == 0 ? gp_Pnt(origin_x_mm + u, origin_y_mm + v, origin_z_mm)
                         : plane == 1 ? gp_Pnt(origin_x_mm + u, origin_y_mm, origin_z_mm + v)
                                      : gp_Pnt(origin_x_mm, origin_y_mm + u, origin_z_mm + v);
      polygon.Add(point);
    }
    polygon.Close();
    if (!polygon.IsDone()) return failure("INVALID_SKETCH", "Polygon outline is not closed");
    BRepBuilderAPI_MakeFace face(polygon.Wire(), true);
    if (!face.IsDone()) return failure("INVALID_SKETCH", "Polygon face could not be built");
    const gp_Vec direction = plane == 0 ? gp_Vec(0, 0, distance_mm)
                             : plane == 1 ? gp_Vec(0, -distance_mm, 0)
                                          : gp_Vec(distance_mm, 0, 0);
    return checked(BRepPrimAPI_MakePrism(face.Face(), direction).Shape());
  } catch (const Standard_Failure& error) {
    return failure("OCCT_ERROR", error.GetMessageString());
  } catch (...) {
    return failure("NATIVE_ERROR", "Polygon extrusion failed");
  }
}

} // namespace forma
