// Exact read-only measurements between two constructor-owned BREP entities.
#include "internal/shape_impl.hpp"
#include "internal/topology_resolver.hpp"
#include "forma_pair_measurement.hpp"
#include <BRepAdaptor_Curve.hxx>
#include <BRepExtrema_DistShapeShape.hxx>
#include <GeomAbs_CurveType.hxx>
#include <Precision.hxx>
#include <Standard_Failure.hxx>
#include <TopoDS.hxx>
#include <gp_Lin.hxx>
#include <gp_Vec.hxx>
#include <algorithm>
#include <cmath>
#include <exception>
#include <stdexcept>

namespace forma {
namespace {
TopAbs_ShapeEnum entity_kind(int kind) {
  if (kind == 0) return TopAbs_EDGE;
  if (kind == 1) return TopAbs_FACE;
  throw std::runtime_error("Pair reference kind must be edge or face");
}
void finite_point(const gp_Pnt& point) {
  if (!std::isfinite(point.X()) || !std::isfinite(point.Y()) || !std::isfinite(point.Z()) ||
      std::abs(point.X()) > 1e9 || std::abs(point.Y()) > 1e9 || std::abs(point.Z()) > 1e9)
    throw std::runtime_error("Exact distance witness is nonfinite or out of bounds");
}
double degrees_from_vectors(const gp_Vec& a, const gp_Vec& b, bool unoriented) {
  double dot = a.Dot(b);
  const double cross = a.Crossed(b).Magnitude();
  if (!std::isfinite(dot) || !std::isfinite(cross))
    throw std::runtime_error("Exact angle products are nonfinite");
  if (unoriented) dot = std::abs(dot);
  // atan2 is stable for coincident/opposed directions; acos amplifies tiny
  // dot-product roundoff near zero and 180 degrees. No near-zero fudge/clamp.
  return std::atan2(cross, dot) * 180.0 / std::acos(-1.0);
}
}
std::unique_ptr<PairMeasurementResult> measure_reference_pair(const Shape& source, int query,
    int kind_a, const std::string& owner_a, const std::string& role_a, const std::string& path_a,
    int kind_b, const std::string& owner_b, const std::string& role_b, const std::string& path_b) noexcept {
  std::unique_ptr<PairMeasurementResult> result;
  try {
    result = std::make_unique<PairMeasurementResult>(); result->kind_ = query;
    const auto fail = [&](const std::string& code, const std::string& detail) {
      result->code_ = code; result->detail_ = detail;
    };
    if (!source.ok() || source.impl_->shape.IsNull()) {
      fail("INVALID_SOURCE", "Pair measurement requires one valid authored body"); return result;
    }
    if (query < 0 || query > 2 || kind_a < 0 || kind_a > 1 || kind_b < 0 || kind_b > 1 ||
        (query == 1 && (kind_a != 1 || kind_b != 1)) || (query == 2 && (kind_a != 0 || kind_b != 0))) {
      fail("INVALID_MEASUREMENT_QUERY", "Invalid pair query or reference-kind combination"); return result;
    }
    const auto a = resolve_topology(source.impl_->shape, source.impl_->topology,
      entity_kind(kind_a), owner_a, role_a, path_a);
    if (!a.valid()) { fail(a.code, a.detail); return result; }
    const auto b = resolve_topology(source.impl_->shape, source.impl_->topology,
      entity_kind(kind_b), owner_b, role_b, path_b);
    if (!b.valid()) { fail(b.code, b.detail); return result; }
    if (query == 0) {
      // Trimmed finite BREP entities, not infinite supporting surfaces/lines.
      BRepExtrema_DistShapeShape extrema;
      extrema.SetMultiThread(false);
      extrema.SetDeflection(Precision::Confusion());
      extrema.LoadS1(a.entity); extrema.LoadS2(b.entity);
      if (!extrema.Perform() || !extrema.IsDone() || extrema.NbSolution() <= 0) {
        fail("MEASUREMENT_FAILED", "OCCT could not resolve exact minimum distance"); return result;
      }
      result->distance_ = extrema.Value();
      if (!std::isfinite(result->distance_) || result->distance_ < 0)
        throw std::runtime_error("Exact minimum distance must be finite and nonnegative");
      // Representative minimizing witness, not a unique/stable topology ID.
      const auto pa = extrema.PointOnShape1(1), pb = extrema.PointOnShape2(1);
      finite_point(pa); finite_point(pb);
      const double tolerance = std::max(1e-6, result->distance_ * 1e-9);
      if (std::abs(pa.Distance(pb) - result->distance_) > tolerance)
        throw std::runtime_error("Distance witness does not match exact scalar");
      result->point_a_ = {pa.X(), pa.Y(), pa.Z()};
      result->point_b_ = {pb.X(), pb.Y(), pb.Z()};
      return result;
    }
    if (query == 1) {
      // Reuse the proven signed outward planar normal implementation, including
      // actual body occurrence orientation and mirrored du-cross-dv frames.
      const auto first = measure_reference(source, 4, owner_a, role_a, path_a);
      if (!first || !first->valid()) {
        fail(first ? first->error_code() : "NATIVE_ALLOCATION_FAILED",
          first ? first->error_message() : "First face measurement allocation failed"); return result;
      }
      const auto second = measure_reference(source, 4, owner_b, role_b, path_b);
      if (!second || !second->valid()) {
        fail(second ? second->error_code() : "NATIVE_ALLOCATION_FAILED",
          second ? second->error_message() : "Second face measurement allocation failed"); return result;
      }
      const auto& na = first->normal(); const auto& nb = second->normal();
      result->angle_ = degrees_from_vectors(gp_Vec(na[0],na[1],na[2]), gp_Vec(nb[0],nb[1],nb[2]), false);
    } else {
      const BRepAdaptor_Curve first(TopoDS::Edge(a.entity)), second(TopoDS::Edge(b.entity));
      if (first.GetType() != GeomAbs_Line || second.GetType() != GeomAbs_Line) {
        fail("MEASUREMENT_UNSUPPORTED", "Acute edge angle requires two exact straight edges"); return result;
      }
      result->angle_ = degrees_from_vectors(gp_Vec(first.Line().Direction()), gp_Vec(second.Line().Direction()), true);
    }
    if (!std::isfinite(result->angle_) || result->angle_ < 0 || result->angle_ > (query == 1 ? 180 : 90))
      throw std::runtime_error("Exact angle is outside its explicit convention");
    return result;
  } catch (const Standard_Failure& error) {
    try { if (result) { result->code_ = "MEASUREMENT_FAILED"; result->detail_ = error.GetMessageString() ? error.GetMessageString() : "OCCT pair measurement exception"; } } catch (...) { return nullptr; }
  } catch (const std::exception& error) {
    try { if (result) { result->code_ = "MEASUREMENT_INVALID_RESULT"; result->detail_ = error.what(); } } catch (...) { return nullptr; }
  } catch (...) {
    try { if (result) { result->code_ = "NATIVE_ERROR"; result->detail_ = "Native pair measurement exception"; } } catch (...) { return nullptr; }
  }
  return result;
}
}
