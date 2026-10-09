#include "internal/shape_impl.hpp"

#include <BRepFilletAPI_MakeChamfer.hxx>
#include <NCollection_IndexedMap.hxx>
#include <Standard_Failure.hxx>
#include <TopAbs_ShapeEnum.hxx>
#include <TopExp.hxx>
#include <TopoDS.hxx>
#include <TopTools_ShapeMapHasher.hxx>

#include <cmath>

namespace forma {

std::unique_ptr<Shape> chamfer_all_edges(const Shape& source, double distance_mm) noexcept {
  if (!source.ok()) return failure("INVALID_SOURCE", "Chamfer input is invalid");
  if (!std::isfinite(distance_mm) || distance_mm <= 0.0 || distance_mm > 10000.0)
    return failure("INVALID_DIMENSION", "Chamfer distance must be positive and finite");
  try {
    NCollection_IndexedMap<TopoDS_Shape, TopTools_ShapeMapHasher> edges;
    TopExp::MapShapes(source.impl_->shape, TopAbs_EDGE, edges);
    if (edges.IsEmpty()) return failure("CHAMFER_NO_EDGES", "Solid has no edges to chamfer");
    BRepFilletAPI_MakeChamfer operation(source.impl_->shape);
    for (int index = 1; index <= edges.Extent(); ++index)
      operation.Add(distance_mm, TopoDS::Edge(edges(index)));
    operation.Build();
    if (!operation.IsDone())
      return failure("CHAMFER_FAILED", "Distance cannot be applied to every edge of this solid");
    return checked(operation.Shape());
  } catch (const Standard_Failure& error) {
    return failure("CHAMFER_FAILED", error.GetMessageString());
  } catch (...) {
    return failure("NATIVE_ERROR", "Chamfer operation failed");
  }
}

} // namespace forma
