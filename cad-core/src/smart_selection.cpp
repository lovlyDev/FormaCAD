#include "internal/shape_impl.hpp"

#include <BRepAdaptor_Curve.hxx>
#include <BRepAdaptor_Surface.hxx>
#include <BRepGProp.hxx>
#include <BRep_Tool.hxx>
#include <GProp_GProps.hxx>
#include <NCollection_IndexedMap.hxx>
#include <TopExp.hxx>
#include <TopExp_Explorer.hxx>
#include <TopTools_ShapeMapHasher.hxx>
#include <TopoDS.hxx>
#include <TopoDS_Edge.hxx>
#include <TopoDS_Face.hxx>
#include <TopoDS_Vertex.hxx>
#include <gp_Circ.hxx>
#include <gp_Cone.hxx>
#include <gp_Cylinder.hxx>
#include <gp_Elips.hxx>
#include <gp_Sphere.hxx>
#include <gp_Torus.hxx>
#include <gp_Vec.hxx>

#include <algorithm>
#include <array>
#include <cmath>
#include <vector>

namespace forma {
namespace {
constexpr std::size_t max_entities = 20'000;
constexpr std::size_t max_selected = 1'024;
using ShapeMap = NCollection_IndexedMap<TopoDS_Shape, TopTools_ShapeMapHasher>;
using Ordinals = std::vector<std::uint32_t>;

bool close_measure(double a, double b) {
  return std::isfinite(a) && std::isfinite(b) &&
    std::abs(a - b) <= std::max(1e-6, 1e-4 * std::max(std::abs(a), std::abs(b)));
}

struct Signature {
  int type;
  double size;
  double radius = 0.0;
  double secondary = 0.0;

  bool matches(const Signature& other) const {
    return type == other.type && close_measure(size, other.size) &&
      close_measure(radius, other.radius) && close_measure(secondary, other.secondary);
  }
};

Signature signature(const TopoDS_Edge& edge) {
  if (BRep_Tool::Degenerated(edge)) return {-1, 0.0};
  BRepAdaptor_Curve curve(edge);
  GProp_GProps properties;
  BRepGProp::LinearProperties(edge, properties);
  Signature result{curve.GetType(), properties.Mass()};
  if (curve.GetType() == GeomAbs_Circle) result.radius = curve.Circle().Radius();
  if (curve.GetType() == GeomAbs_Ellipse) {
    result.radius = curve.Ellipse().MajorRadius();
    result.secondary = curve.Ellipse().MinorRadius();
  }
  return result;
}

Signature signature(const TopoDS_Face& face) {
  BRepAdaptor_Surface surface(face, true);
  GProp_GProps properties;
  BRepGProp::SurfaceProperties(face, properties);
  Signature result{surface.GetType(), properties.Mass()};
  switch (surface.GetType()) {
    case GeomAbs_Cylinder: result.radius = surface.Cylinder().Radius(); break;
    case GeomAbs_Sphere: result.radius = surface.Sphere().Radius(); break;
    case GeomAbs_Cone:
      result.radius = surface.Cone().RefRadius();
      result.secondary = std::abs(surface.Cone().SemiAngle());
      break;
    case GeomAbs_Torus:
      result.radius = surface.Torus().MajorRadius();
      result.secondary = surface.Torus().MinorRadius();
      break;
    default: break;
  }
  return result;
}

template<class Entity>
Ordinals similar(const std::vector<Entity>& entities, std::uint32_t seed) {
  const auto reference = signature(entities[seed - 1]);
  if (!(reference.size > 0.0)) return {};
  Ordinals result;
  for (std::size_t i = 0; i < entities.size(); ++i) {
    if (!reference.matches(signature(entities[i]))) continue;
    if (result.size() == max_selected) return {};
    result.push_back(static_cast<std::uint32_t>(i + 1));
  }
  return result;
}

struct End {
  std::size_t edge;
  int side;
  gp_Vec away;
};

Ordinals tangent_chain(const std::vector<TopoDS_Edge>& edges, std::uint32_t seed) {
  ShapeMap vertices;
  std::vector<std::vector<End>> junctions;
  std::vector<std::array<int, 2>> at(edges.size());
  for (std::size_t i = 0; i < edges.size(); ++i) {
    if (BRep_Tool::Degenerated(edges[i])) continue;
    const auto edge = TopoDS::Edge(edges[i].Oriented(TopAbs_FORWARD));
    TopoDS_Vertex start, end;
    TopExp::Vertices(edge, start, end);
    if (start.IsNull() || end.IsNull()) continue;
    BRepAdaptor_Curve curve(edge);
    for (int side = 0; side < 2; ++side) {
      gp_Pnt point;
      gp_Vec derivative;
      curve.D1(side == 0 ? curve.FirstParameter() : curve.LastParameter(), point, derivative);
      if (!(derivative.SquareMagnitude() > 1e-20)) continue;
      derivative.Normalize();
      if (side == 1) derivative.Reverse();
      const int vertex = vertices.Add(side == 0 ? start : end);
      junctions.resize(static_cast<std::size_t>(vertices.Extent()));
      junctions[vertex - 1].push_back({i, side, derivative});
      at[i][side] = vertex;
    }
  }
  if (!at[seed - 1][0] && !at[seed - 1][1]) return {};
  std::vector<bool> visited(edges.size());
  Ordinals result{seed};
  visited[seed - 1] = true;
  for (std::size_t i = 0; i < result.size(); ++i) {
    const std::size_t index = result[i] - 1;
    for (int side = 0; side < 2; ++side) {
      const int vertex = at[index][side];
      if (!vertex) continue;
      const auto& ends = junctions[vertex - 1];
      const auto current = std::find_if(ends.begin(), ends.end(), [&](const End& end) {
        return end.edge == index && end.side == side;
      });
      if (current == ends.end()) continue;
      std::size_t other = edges.size();
      for (const auto& end : ends) {
        if (end.edge == index || current->away.Dot(end.away) > -std::cos(0.001)) continue;
        if (other != edges.size() && other != end.edge) {
          other = edges.size();
          break;
        }
        other = end.edge;
      }
      if (other == edges.size() || visited[other]) continue;
      if (result.size() == max_selected) return {};
      visited[other] = true;
      result.push_back(static_cast<std::uint32_t>(other + 1));
    }
  }
  std::sort(result.begin(), result.end());
  return result;
}
}

std::unique_ptr<std::vector<std::uint32_t>> smart_select(
    const Shape& source, int kind, std::uint32_t seed) noexcept {
  try {
    if (!source.ok() || seed == 0) return nullptr;
    const auto& shape = source.impl_->shape;
    if (kind == 0) {
      // Preserve the exact order used by the committed GLB face ranges.
      std::vector<TopoDS_Face> faces;
      for (TopExp_Explorer it(shape, TopAbs_FACE); it.More(); it.Next()) {
        if (faces.size() == max_entities) return nullptr;
        faces.push_back(TopoDS::Face(it.Current()));
      }
      if (seed > faces.size()) return nullptr;
      return std::make_unique<Ordinals>(similar(faces, seed));
    }
    if (kind != 1 && kind != 2) return nullptr;
    ShapeMap map;
    TopExp::MapShapes(shape, TopAbs_EDGE, map);
    if (static_cast<std::size_t>(map.Extent()) > max_entities ||
        seed > static_cast<std::uint32_t>(map.Extent())) return nullptr;
    std::vector<TopoDS_Edge> edges;
    edges.reserve(map.Extent());
    for (int i = 1; i <= map.Extent(); ++i) edges.push_back(TopoDS::Edge(map(i)));
    return std::make_unique<Ordinals>(kind == 1 ? similar(edges, seed) : tangent_chain(edges, seed));
  } catch (...) {
    return nullptr;
  }
}
}
