#include "internal/shape_impl.hpp"
#include "internal/topology_resolver.hpp"
#include <BRepAdaptor_Curve.hxx>
#include <BRepAdaptor_Surface.hxx>
#include <GCPnts_AbscissaPoint.hxx>
#include <IntCurvesFace_ShapeIntersector.hxx>
#include <Standard_Failure.hxx>
#include <TopoDS.hxx>
#include <gp_Circ.hxx>
#include <gp_Dir.hxx>
#include <gp_Lin.hxx>
#include <gp_Vec.hxx>
#include <cmath>
#include <limits>
#include <stdexcept>

namespace forma {
std::unique_ptr<SnapResult> snap_reference(const Shape& source, int kind,
    const std::string& owner, const std::string& role, const std::string& path,
    const double* ray, std::size_t ray_count) noexcept {
  std::unique_ptr<SnapResult> result;
  try {
    result = std::make_unique<SnapResult>(); result->kind_ = kind;
    const auto fail = [&](const std::string& code, const std::string& detail) {
      result->code_ = code; result->detail_ = detail;
    };
    if (!source.ok() || source.impl_->shape.IsNull()) {
      fail("INVALID_SOURCE", "Snapping requires a valid source BREP"); return result;
    }
    if (kind < 0 || kind > 2 || (kind == 2 ? !ray || ray_count != 6 : ray_count != 0)) {
      fail("INVALID_SNAP_QUERY", "Invalid snap mode or ray shape"); return result;
    }
    if (kind == 2) {
      for (std::size_t i = 0; i < ray_count; ++i) {
        if (!std::isfinite(ray[i]) || std::abs(ray[i]) > (i < 3 ? 1e9 : 1e6)) {
          fail("INVALID_SNAP_QUERY", "Snap ray must be finite and bounded"); return result;
        }
      }
      if (std::hypot(ray[3], ray[4], ray[5]) < 1e-9) {
        fail("INVALID_SNAP_QUERY", "Snap ray direction is zero"); return result;
      }
    }
    const auto resolved = resolve_topology(source.impl_->shape, source.impl_->topology,
      kind == 2 ? TopAbs_FACE : TopAbs_EDGE, owner, role, path);
    if (!resolved.valid()) { fail(resolved.code, resolved.detail); return result; }
    gp_Pnt point;
    if (kind < 2) {
      BRepAdaptor_Curve curve(TopoDS::Edge(resolved.entity));
      if (kind == 1) {
        if (curve.GetType() != GeomAbs_Circle) {
          fail("SNAP_UNSUPPORTED", "Circle centre requires an analytic circular edge"); return result;
        }
        point = curve.Circle().Location();
        const auto normal = curve.Circle().Axis().Direction();
        result->normal_ = {normal.X(), normal.Y(), normal.Z()};
      } else {
        const double first = curve.FirstParameter(), last = curve.LastParameter();
        const double length = GCPnts_AbscissaPoint::Length(curve, first, last, 1e-9);
        if (!std::isfinite(length) || length <= 1e-9) {
          fail("SNAP_UNSUPPORTED", "Edge has no usable arc length"); return result;
        }
        GCPnts_AbscissaPoint midpoint(1e-9, curve, length / 2, first);
        if (!midpoint.IsDone() || !std::isfinite(midpoint.Parameter()) || midpoint.Parameter() < first - 1e-9 || midpoint.Parameter() > last + 1e-9) {
          fail("SNAP_NO_TARGET", "Edge midpoint could not be resolved"); return result;
        }
        point = curve.Value(midpoint.Parameter());
      }
    } else {
      const auto face = TopoDS::Face(resolved.entity);
      if (face.Orientation() != TopAbs_FORWARD && face.Orientation() != TopAbs_REVERSED) {
        fail("SNAP_UNSUPPORTED", "Face has no supported outward orientation"); return result;
      }
      IntCurvesFace_ShapeIntersector intersections;
      intersections.Load(face, 1e-7);
      intersections.Perform(gp_Lin(gp_Pnt(ray[0], ray[1], ray[2]), gp_Dir(ray[3], ray[4], ray[5])), 0., 4e9);
      int nearest = 0; double distance = std::numeric_limits<double>::infinity();
      if (intersections.IsDone()) for (int i = 1; i <= intersections.NbPnt(); ++i) {
        const double parameter = intersections.WParameter(i);
        if (std::isfinite(parameter) && parameter >= 0 && parameter < distance) { nearest = i; distance = parameter; }
      }
      if (!nearest) { fail("SNAP_NO_TARGET", "Ray misses the selected trimmed face"); return result; }
      point = intersections.Pnt(nearest);
      BRepAdaptor_Surface surface(face, true);
      gp_Pnt evaluated; gp_Vec du, dv;
      surface.D1(intersections.UParameter(nearest), intersections.VParameter(nearest), evaluated, du, dv);
      auto normal = du.Crossed(dv);
      if (!std::isfinite(normal.SquareMagnitude()) || normal.SquareMagnitude() < 1e-18) {
        fail("SNAP_UNSUPPORTED", "Surface normal is singular at the snap point"); return result;
      }
      normal.Normalize();
      if (face.Orientation() == TopAbs_REVERSED) normal.Reverse();
      result->normal_ = {normal.X(), normal.Y(), normal.Z()};
    }
    result->point_ = {point.X(), point.Y(), point.Z()};
    for (const double coordinate : result->point_) if (!std::isfinite(coordinate) || std::abs(coordinate) > 1e9)
      throw std::runtime_error("Snap point is non-finite or outside supported bounds");
    if (!result->normal_.empty()) {
      const double norm = std::hypot(result->normal_[0], result->normal_[1], result->normal_[2]);
      if (!std::isfinite(norm) || std::abs(norm - 1.) > 1e-8) throw std::runtime_error("Snap normal is not unit length");
    }
    return result;
  } catch (const Standard_Failure& error) {
    try { if (result) { result->code_ = "SNAP_FAILED"; result->detail_ = error.GetMessageString() ? error.GetMessageString() : "OCCT snap exception"; } } catch (...) { return nullptr; }
  } catch (const std::exception& error) {
    try { if (result) { result->code_ = "SNAP_FAILED"; result->detail_ = error.what(); } } catch (...) { return nullptr; }
  } catch (...) {
    try { if (result) { result->code_ = "SNAP_FAILED"; result->detail_ = "Native snap exception"; } } catch (...) { return nullptr; }
  }
  return result;
}
} // namespace forma
