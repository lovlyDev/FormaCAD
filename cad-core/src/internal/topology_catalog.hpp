#pragma once
#include <TopoDS_Shape.hxx>
#include <BRepBuilderAPI_Transform.hxx>
#include <string>
#include <vector>

namespace forma {
struct TopologyEntry {
  TopoDS_Shape entity;
  std::string owner;
  std::string role;
  std::vector<std::string> path;
};
using TopologyCatalog = std::vector<TopologyEntry>;
TopologyCatalog transformed_catalog(const TopologyCatalog&, BRepBuilderAPI_Transform&,
                                    const TopoDS_Shape&);
// Exact BREP membership and unique identity; null is explicit unsupported/ambiguous.
std::string topology_reference_json(const TopologyCatalog&, const TopoDS_Shape&);
} // namespace forma
