#include "internal/face_measurements.hpp"

#include <BRepGProp.hxx>
#include <GProp_GProps.hxx>

#include <cmath>
#include <stdexcept>

namespace forma {

double face_area_mm2(const TopoDS_Face& face) {
  GProp_GProps properties;
  BRepGProp::SurfaceProperties(face, properties);
  const double area = properties.Mass();
  if (!std::isfinite(area) || area < 0.0) {
    throw std::runtime_error("CAD face area is invalid");
  }
  return area;
}

} // namespace forma
