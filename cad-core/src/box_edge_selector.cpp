#include "internal/box_edge_selector.hpp"

#include <BRepAdaptor_Curve.hxx>
#include <BRepBndLib.hxx>
#include <Bnd_Box.hxx>
#include <GeomAbs_CurveType.hxx>
#include <NCollection_IndexedMap.hxx>
#include <TopAbs_ShapeEnum.hxx>
#include <TopExp.hxx>
#include <TopTools_ShapeMapHasher.hxx>
#include <TopoDS.hxx>
#include <TopoDS_Edge.hxx>
#include <Standard_Failure.hxx>

#include <algorithm>
#include <array>
#include <cmath>
#include <unordered_map>

namespace forma {
namespace {

std::string candidate_key(const TopoDS_Edge& edge,
                          const std::array<double, 3>& low,
                          const std::array<double, 3>& high) {
  BRepAdaptor_Curve curve(edge);
  if (curve.GetType() != GeomAbs_Line) return {};
  const gp_Pnt first = curve.Value(curve.FirstParameter());
  const gp_Pnt last = curve.Value(curve.LastParameter());
  const std::array<double, 3> a{first.X(), first.Y(), first.Z()};
  const std::array<double, 3> b{last.X(), last.Y(), last.Z()};
  const char* axes[] = {"x", "y", "z"};
  for (int varying = 0; varying < 3; ++varying) {
    const double span = high[varying] - low[varying];
    const double tolerance = std::max(0.0001, span * 0.00001);
    if (span <= tolerance * 2) continue;
    if (!(std::abs(a[varying] - low[varying]) <= tolerance &&
          std::abs(b[varying] - high[varying]) <= tolerance) &&
        !(std::abs(b[varying] - low[varying]) <= tolerance &&
          std::abs(a[varying] - high[varying]) <= tolerance)) continue;
    std::string key = std::string("box-edge:") + axes[varying];
    bool valid = true;
    for (int fixed = 0; fixed < 3; ++fixed) {
      if (fixed == varying) continue;
      const double fixed_tolerance = std::max(0.0001, (high[fixed] - low[fixed]) * 0.00001);
      if (std::abs(a[fixed] - low[fixed]) <= fixed_tolerance &&
          std::abs(b[fixed] - low[fixed]) <= fixed_tolerance)
        key += std::string(":") + axes[fixed] + "min";
      else if (std::abs(a[fixed] - high[fixed]) <= fixed_tolerance &&
               std::abs(b[fixed] - high[fixed]) <= fixed_tolerance)
        key += std::string(":") + axes[fixed] + "max";
      else valid = false;
    }
    if (valid) return key;
  }
  return {};
}

} // namespace

std::vector<std::string> box_edge_keys(const TopoDS_Shape& shape) {
  NCollection_IndexedMap<TopoDS_Shape, TopTools_ShapeMapHasher> edges;
  TopExp::MapShapes(shape, TopAbs_EDGE, edges);
  std::vector<std::string> keys(edges.Extent());
  Bnd_Box bounds;
  BRepBndLib::Add(shape, bounds);
  if (bounds.IsVoid()) return keys;
  double xmin, ymin, zmin, xmax, ymax, zmax;
  bounds.Get(xmin, ymin, zmin, xmax, ymax, zmax);
  const std::array<double, 3> low{xmin, ymin, zmin};
  const std::array<double, 3> high{xmax, ymax, zmax};
  if (!std::all_of(low.begin(), low.end(), [](double n) { return std::isfinite(n); }) ||
      !std::all_of(high.begin(), high.end(), [](double n) { return std::isfinite(n); })) return keys;
  std::unordered_map<std::string, int> counts;
  for (int index = 1; index <= edges.Extent(); ++index) {
    try {
      keys[index - 1] = candidate_key(TopoDS::Edge(edges(index)), low, high);
    } catch (const Standard_Failure&) {
      continue;
    }
    if (!keys[index - 1].empty()) ++counts[keys[index - 1]];
  }
  for (auto& key : keys) {
    if (!key.empty() && counts[key] != 1) key.clear();
  }
  return keys;
}

} // namespace forma
