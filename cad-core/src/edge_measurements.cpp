#include "internal/edge_measurements.hpp"
#include "internal/box_edge_selector.hpp"

#include <BRepAdaptor_Curve.hxx>
#include <BRepGProp.hxx>
#include <GProp_GProps.hxx>
#include <GeomAbs_CurveType.hxx>
#include <NCollection_IndexedMap.hxx>
#include <TopAbs_ShapeEnum.hxx>
#include <TopExp.hxx>
#include <TopTools_ShapeMapHasher.hxx>
#include <TopoDS.hxx>
#include <TopoDS_Edge.hxx>

#include <cmath>
#include <stdexcept>

namespace forma {
namespace {
constexpr std::size_t max_preview_points = 200'000;

std::array<double, 3> gltf_point(const gp_Pnt& point) {
  if (!std::isfinite(point.X()) || !std::isfinite(point.Y()) ||
      !std::isfinite(point.Z())) {
    throw std::runtime_error("CAD edge point is invalid");
  }
  return {point.X() / 1000.0, point.Z() / 1000.0, -point.Y() / 1000.0};
}
} // namespace

std::vector<PreviewEdge> preview_edges(const TopoDS_Shape& shape, const TopologyCatalog& catalog) {
  std::vector<PreviewEdge> result;
  std::size_t total_points = 0;
  NCollection_IndexedMap<TopoDS_Shape, TopTools_ShapeMapHasher> edges;
  TopExp::MapShapes(shape, TopAbs_EDGE, edges);
  const auto semantic_keys = box_edge_keys(shape);
  result.reserve(edges.Extent());
  for (int index = 1; index <= edges.Extent(); ++index) {
    const TopoDS_Edge edge = TopoDS::Edge(edges.FindKey(index));
    GProp_GProps properties;
    BRepGProp::LinearProperties(edge, properties);
    const double length = properties.Mass();
    if (!std::isfinite(length) || length < 0.0) {
      throw std::runtime_error("CAD edge length is invalid");
    }
    PreviewEdge preview{length, std::nullopt, {}, semantic_keys[index - 1], topology_reference_json(catalog, edge)};
    if (length > 0.0 && total_points < max_preview_points) {
      BRepAdaptor_Curve curve(edge);
      if (curve.GetType() == GeomAbs_Circle) {
        const double radius = curve.Circle().Radius();
        if (!std::isfinite(radius) || radius <= 0.0) {
          throw std::runtime_error("CAD edge radius is invalid");
        }
        preview.radius_mm = radius;
      }
      const double first = curve.FirstParameter();
      const double last = curve.LastParameter();
      if (std::isfinite(first) && std::isfinite(last) && last > first) {
        const int segments = curve.GetType() == GeomAbs_Line ? 1 : 48;
        if (total_points + static_cast<std::size_t>(segments + 1) <= max_preview_points) {
          preview.points_m.reserve(segments + 1);
          for (int segment = 0; segment <= segments; ++segment) {
            const double parameter = first + (last - first) * segment / segments;
            preview.points_m.push_back(gltf_point(curve.Value(parameter)));
          }
          total_points += preview.points_m.size();
        }
      }
    }
    result.push_back(std::move(preview));
  }
  return result;
}

} // namespace forma
