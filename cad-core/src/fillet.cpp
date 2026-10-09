#include "internal/shape_impl.hpp"
#include "internal/box_edge_selector.hpp"

#include <BRepFilletAPI_MakeFillet.hxx>
#include <Standard_Failure.hxx>
#include <TopAbs_ShapeEnum.hxx>
#include <TopExp.hxx>
#include <TopoDS.hxx>
#include <NCollection_IndexedMap.hxx>
#include <TopTools_ShapeMapHasher.hxx>

#include <cmath>

namespace forma {

std::unique_ptr<Shape> fillet_all_edges(const Shape& source, double radius_mm) noexcept {
  if (!source.ok()) return failure("INVALID_SOURCE", "Fillet input is invalid");
  if (!std::isfinite(radius_mm) || radius_mm <= 0.0 || radius_mm > 10000.0)
    return failure("INVALID_DIMENSION", "Fillet radius must be positive and finite");
  try {
    NCollection_IndexedMap<TopoDS_Shape, TopTools_ShapeMapHasher> edges;
    TopExp::MapShapes(source.impl_->shape, TopAbs_EDGE, edges);
    if (edges.IsEmpty()) return failure("FILLET_NO_EDGES", "Solid has no edges to fillet");
    BRepFilletAPI_MakeFillet operation(source.impl_->shape);
    for (int index = 1; index <= edges.Extent(); ++index)
      operation.Add(radius_mm, TopoDS::Edge(edges(index)));
    operation.Build();
    if (!operation.IsDone())
      return failure("FILLET_FAILED", "Radius cannot be applied to every edge of this solid");
    return checked(operation.Shape());
  } catch (const Standard_Failure& error) {
    return failure("FILLET_FAILED", error.GetMessageString());
  } catch (...) {
    return failure("NATIVE_ERROR", "Fillet operation failed");
  }
}

std::unique_ptr<Shape> fillet_edge(const Shape& source, const std::string& edge_key,
                                   double radius_mm) noexcept {
  if (!source.ok()) return failure("INVALID_SOURCE", "Fillet input is invalid");
  if (!std::isfinite(radius_mm) || radius_mm <= 0.0 || radius_mm > 10000.0)
    return failure("INVALID_DIMENSION", "Fillet radius must be positive and finite");
  if (edge_key.size() > 64 || !edge_key.starts_with("box-edge:"))
    return failure("INVALID_EDGE_REFERENCE", "Unsupported edge selector");
  try {
    const auto keys = box_edge_keys(source.impl_->shape);
    NCollection_IndexedMap<TopoDS_Shape, TopTools_ShapeMapHasher> edges;
    TopExp::MapShapes(source.impl_->shape, TopAbs_EDGE, edges);
    for (int index = 1; index <= edges.Extent(); ++index) {
      if (keys[index - 1] != edge_key) continue;
      BRepFilletAPI_MakeFillet operation(source.impl_->shape);
      operation.Add(radius_mm, TopoDS::Edge(edges(index)));
      operation.Build();
      if (!operation.IsDone())
        return failure("FILLET_FAILED", "Radius cannot be applied to the selected edge");
      return checked(operation.Shape());
    }
    return failure("EDGE_REFERENCE_UNRESOLVED", "Selected edge is missing or ambiguous");
  } catch (const Standard_Failure& error) {
    return failure("FILLET_FAILED", error.GetMessageString());
  } catch (...) {
    return failure("NATIVE_ERROR", "Selected-edge fillet failed");
  }
}

} // namespace forma
