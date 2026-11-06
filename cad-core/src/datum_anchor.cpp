#include "forma_datum_anchor.hpp"
#include "internal/shape_impl.hpp"
#include "internal/topology_resolver.hpp"

#include <BRepAdaptor_Curve.hxx>
#include <BRepAdaptor_Surface.hxx>
#include <BRepGProp.hxx>
#include <GProp_GProps.hxx>
#include <TopoDS.hxx>
#include <gp_Lin.hxx>
#include <gp_Pln.hxx>
#include <algorithm>
#include <array>
#include <cmath>

namespace forma {
namespace {

bool face_frame(const TopoDS_Shape& entity, gp_Pnt& origin, gp_Dir& direction) {
  const auto face = TopoDS::Face(entity);
  const BRepAdaptor_Surface surface(face);
  if (surface.GetType() != GeomAbs_Plane) return false;
  GProp_GProps properties;
  BRepGProp::SurfaceProperties(face, properties);
  if (!std::isfinite(properties.Mass()) || properties.Mass() <= 0.0) return false;
  origin = properties.CentreOfMass();
  direction = surface.Plane().Axis().Direction();
  if (face.Orientation() == TopAbs_REVERSED) direction.Reverse();
  return true;
}

bool edge_frame(const TopoDS_Shape& entity, gp_Pnt& origin, gp_Dir& direction) {
  const auto edge = TopoDS::Edge(entity);
  const BRepAdaptor_Curve curve(edge);
  if (curve.GetType() != GeomAbs_Line) return false;
  const double first = curve.FirstParameter();
  const double last = curve.LastParameter();
  if (!std::isfinite(first) || !std::isfinite(last) || first >= last) return false;
  origin = curve.Value(first + (last - first) / 2.0);
  direction = curve.Line().Direction();
  if (edge.Orientation() == TopAbs_REVERSED) direction.Reverse();
  return true;
}

} // namespace

bool datum_anchor(const Shape& solid, int kind, const std::string& owner,
                  const std::string& role, const std::string& occurrence,
                  double* frame, std::size_t count) noexcept {
  if (!solid.ok() || (kind != 0 && kind != 1) || !frame || count != 6) return false;
  try {
    const auto result = resolve_topology(solid.impl_->shape, solid.impl_->topology,
      kind == 0 ? TopAbs_FACE : TopAbs_EDGE, owner, role, occurrence);
    if (!result.valid()) return false;
    gp_Pnt origin;
    gp_Dir direction;
    if (!(kind == 0 ? face_frame(result.entity, origin, direction)
                   : edge_frame(result.entity, origin, direction))) return false;
    const std::array values{origin.X(), origin.Y(), origin.Z(),
                            direction.X(), direction.Y(), direction.Z()};
    if (!std::all_of(values.begin(), values.end(), [](double value) { return std::isfinite(value); }))
      return false;
    std::copy(values.begin(), values.end(), frame);
    return true;
  } catch (...) {
    return false;
  }
}

} // namespace forma
