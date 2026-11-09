#include "internal/shape_impl.hpp"
#include "internal/topology_resolver.hpp"

#include <BRepFilletAPI_MakeChamfer.hxx>
#include <BRepFilletAPI_MakeFillet.hxx>
#include <Standard_Failure.hxx>
#include <TopoDS.hxx>
#include <TopoDS_Edge.hxx>

#include <algorithm>
#include <cmath>
#include <sstream>
#include <vector>

namespace forma {
std::unique_ptr<Shape> blend_referenced_edges(const Shape& source, const std::string& owner,
    const std::string& role, const std::string& path, const std::string& additional,
    double size, bool chamfer) noexcept {
  const char* code = chamfer ? "CHAMFER_FAILED" : "FILLET_FAILED";
  if (!source.ok()) return failure("INVALID_SOURCE", "Invalid edge blend source");
  if (!std::isfinite(size) || size <= 0 || size > 10000)
    return failure("INVALID_DIMENSION", "Invalid edge blend dimension");
  constexpr std::size_t max_edges = 64;
  constexpr std::size_t max_reference_bytes = 80 + 80 + 64 * 81 + 2;
  if (additional.size() > (max_edges - 1) * max_reference_bytes)
    return failure("INVALID_TOPOLOGY_REFERENCE", "Edge reference packet is too large");
  try {
    const auto& shape = source.impl_->shape;
    const auto& catalog = source.impl_->topology;
    const auto first = resolve_topology(shape, catalog, TopAbs_EDGE, owner, role, path);
    if (!first.valid()) return failure(first.code.c_str(), first.detail.c_str());
    std::vector<TopoDS_Edge> edges{TopoDS::Edge(first.entity)};
    std::istringstream records(additional);
    std::string record;
    while (std::getline(records, record)) {
      const auto end_owner = record.find('\t');
      const auto end_role = end_owner == std::string::npos ? end_owner : record.find('\t', end_owner + 1);
      if (edges.size() == max_edges || end_role == std::string::npos)
        return failure("INVALID_TOPOLOGY_REFERENCE", "Invalid edge reference packet");
      const auto resolved = resolve_topology(shape, catalog, TopAbs_EDGE, record.substr(0, end_owner),
        record.substr(end_owner + 1, end_role - end_owner - 1), record.substr(end_role + 1));
      if (!resolved.valid()) return failure(resolved.code.c_str(), resolved.detail.c_str());
      if (std::any_of(edges.begin(), edges.end(), [&](const auto& edge) { return edge.IsSame(resolved.entity); }))
        return failure("INVALID_TOPOLOGY_REFERENCE", "Edge selection contains a duplicate");
      edges.push_back(TopoDS::Edge(resolved.entity));
    }
    const auto build = [&](auto& operation) -> std::unique_ptr<Shape> {
      for (const auto& edge : edges) {
        // OCCT adds a whole tangent contour from its first edge.
        if (operation.Contour(edge) == 0) operation.Add(size, edge);
        if (operation.Contour(edge) == 0)
          return failure(code, "Selected edge cannot start an edge blend contour");
      }
      operation.Build();
      if (!operation.IsDone()) return failure(code, "Size cannot be applied to the selected edges");
      return checked(operation.Shape());
    };
    if (chamfer) {
      BRepFilletAPI_MakeChamfer operation(shape);
      return build(operation);
    }
    BRepFilletAPI_MakeFillet operation(shape);
    return build(operation);
  } catch (const Standard_Failure& error) {
    return failure(code, error.GetMessageString());
  } catch (...) {
    return failure("NATIVE_ERROR", "Edge blend failed");
  }
}
}
