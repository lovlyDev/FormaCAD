#pragma once

#include <TopoDS_Shape.hxx>
#include "topology_catalog.hpp"

#include <array>
#include <optional>
#include <string>
#include <vector>

namespace forma {

struct PreviewEdge {
  double length_mm;
  std::optional<double> radius_mm;
  std::vector<std::array<double, 3>> points_m;
  std::string semantic_key;
  std::string topology_ref;
};

// Unique TopExp::MapShapes order matches the edge count used by model inspection.
std::vector<PreviewEdge> preview_edges(const TopoDS_Shape& shape, const TopologyCatalog& catalog = {});

} // namespace forma
