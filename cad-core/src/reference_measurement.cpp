#include "internal/shape_impl.hpp"
#include "internal/topology_resolver.hpp"
#include "forma_measurement.hpp"
#include <BRepAdaptor_Curve.hxx>
#include <BRepAdaptor_Surface.hxx>
#include <BRepGProp.hxx>
#include <GProp_GProps.hxx>
#include <GeomAbs_CurveType.hxx>
#include <GeomAbs_SurfaceType.hxx>
#include <Standard_Failure.hxx>
#include <TopoDS.hxx>
#include <gp_Pln.hxx>
#include <gp_Circ.hxx>
#include <gp_Vec.hxx>
#include <cmath>
#include <exception>
#include <stdexcept>

namespace forma {
namespace {
void positive(double value) {
  if (!std::isfinite(value) || value <= 0)
    throw std::runtime_error("Kernel measurement must be finite and positive");
}
void finite_vector(const std::vector<double>& value) {
  if (value.size() != 3) throw std::runtime_error("Kernel vector must have three coordinates");
  for (double coordinate : value)
    if (!std::isfinite(coordinate)) throw std::runtime_error("Kernel vector is non-finite");
}
}
std::unique_ptr<MeasurementResult> measure_reference(const Shape& source, int kind,
    const std::string& owner, const std::string& role, const std::string& path) noexcept {
  std::unique_ptr<MeasurementResult> result;
  try {
    result = std::make_unique<MeasurementResult>(); result->kind_ = kind;
    const auto fail = [&](const std::string& code, const std::string& detail) {
      result->code_ = code; result->detail_ = detail;
    };
    if (!source.ok() || source.impl_->shape.IsNull()) { fail("INVALID_SOURCE", "Invalid measurement body"); return result; }
    if (kind < 0 || kind > 5 || (kind == 0 && (!owner.empty() || !role.empty() || !path.empty()))) {
      fail("INVALID_MEASUREMENT_QUERY", "Invalid query kind or body query reference"); return result;
    }
    if (kind == 0) {
      result->volume_ = source.volume_mm3(); result->area_ = source.surface_area_mm2();
      result->faces_ = source.face_count(); result->edges_ = source.edge_count();
      result->extents_ = {source.extent_x_mm(), source.extent_y_mm(), source.extent_z_mm()};
      positive(result->volume_); positive(result->area_); finite_vector(result->extents_);
      for (double extent : result->extents_) positive(extent);
      if (result->faces_ <= 0 || result->edges_ <= 0)
        throw std::runtime_error("Kernel body counts are invalid");
      return result;
    }
    const auto resolved = resolve_topology(source.impl_->shape, source.impl_->topology,
      kind <= 2 || kind == 5 ? TopAbs_EDGE : TopAbs_FACE, owner, role, path);
    if (!resolved.valid()) { fail(resolved.code, resolved.detail); return result; }
    GProp_GProps properties;
    if (kind <= 2 || kind == 5) {
      const auto edge = TopoDS::Edge(resolved.entity);
      if (kind == 1) {
        BRepGProp::LinearProperties(edge, properties); result->length_ = properties.Mass();
        positive(result->length_);
      } else {
        BRepAdaptor_Curve curve(edge);
        if (curve.GetType() != GeomAbs_Circle) {
          fail("MEASUREMENT_UNSUPPORTED", "Radius requires an exact circular edge"); return result;
        }
        result->radius_ = curve.Circle().Radius(); positive(result->radius_);
        if (kind == 5) { result->diameter_ = 2 * result->radius_; positive(result->diameter_); }
      }
      return result;
    }
    const auto face = TopoDS::Face(resolved.entity);
    BRepGProp::SurfaceProperties(face, properties); result->area_ = properties.Mass(); positive(result->area_);
    if (kind == 4) {
      BRepAdaptor_Surface surface(face, true);
      if (surface.GetType() != GeomAbs_Plane) {
        fail("MEASUREMENT_UNSUPPORTED", "Selected face is not planar"); return result;
      }
      if (face.Orientation() != TopAbs_FORWARD && face.Orientation() != TopAbs_REVERSED) {
        fail("MEASUREMENT_UNSUPPORTED", "Face has no supported outward orientation"); return result;
      }
      const auto origin = properties.CentreOfMass();
      // Plane.Axis is an axial direction. A mirrored plane can have a
      // left-handed parameter frame; its oriented normal is du cross dv.
      gp_Pnt surface_point;
      gp_Vec tangent_u, tangent_v;
      surface.D1(0.0, 0.0, surface_point, tangent_u, tangent_v);
      auto normal = gp_Dir(tangent_u.Crossed(tangent_v));
      if (face.Orientation() == TopAbs_REVERSED) normal.Reverse();
      result->origin_ = {origin.X(), origin.Y(), origin.Z()};
      result->normal_ = {normal.X(), normal.Y(), normal.Z()};
      finite_vector(result->origin_); finite_vector(result->normal_);
      const double norm = std::hypot(normal.X(), normal.Y(), normal.Z());
      if (std::abs(norm - 1.0) > 1e-10) throw std::runtime_error("Kernel normal is not unit length");
    }
    return result;
  } catch (const Standard_Failure& error) {
    try { if (result) { result->code_ = "MEASUREMENT_FAILED"; result->detail_ = error.GetMessageString() ? error.GetMessageString() : "OCCT measurement exception"; } } catch (...) { return nullptr; }
  } catch (const std::exception& error) {
    try { if (result) { result->code_ = "MEASUREMENT_INVALID_RESULT"; result->detail_ = error.what(); } } catch (...) { return nullptr; }
  } catch (...) {
    try { if (result) { result->code_ = "NATIVE_ERROR"; result->detail_ = "Native measurement exception"; } } catch (...) { return nullptr; }
  }
  return result; // Null allocation must be rejected by the Rust boundary.
}
}
