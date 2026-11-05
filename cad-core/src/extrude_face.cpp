#include "internal/shape_impl.hpp"
#include "internal/topology_resolver.hpp"

#include <BRepAdaptor_Surface.hxx>
#include <BRepFeat_MakePrism.hxx>
#include <NCollection_IndexedMap.hxx>
#include <Precision.hxx>
#include <Standard_Failure.hxx>
#include <TopExp.hxx>
#include <TopTools_ShapeMapHasher.hxx>
#include <TopoDS.hxx>
#include <gp_Pln.hxx>
#include <cmath>

namespace forma {

std::unique_ptr<Shape> extrude_face(const Shape& source, const std::string& owner,
                                   const std::string& role, const std::string& occurrence,
                                   double distance_mm) noexcept {
  if (!source.ok()) return failure("INVALID_SOURCE", "Face extrusion requires a valid solid");
  if (!std::isfinite(distance_mm) || std::abs(distance_mm) <= Precision::Confusion() ||
      std::abs(distance_mm) > 10000.0)
    return failure("INVALID_DIMENSION", "Face extrusion distance is invalid");
  try {
    const auto& solid = source.impl_->shape;
    NCollection_IndexedMap<TopoDS_Shape, TopTools_ShapeMapHasher> solids;
    TopExp::MapShapes(solid, TopAbs_SOLID, solids);
    if (solids.Extent() != 1)
      return failure("EXTRUDE_FACE_FAILED", "Face extrusion requires exactly one solid");
    const auto reference = resolve_topology(solid, source.impl_->topology, TopAbs_FACE, owner, role, occurrence);
    if (!reference.valid()) return failure(reference.code.c_str(), reference.detail.c_str());
    const auto face = TopoDS::Face(reference.entity);
    const BRepAdaptor_Surface surface(face);
    if (surface.GetType() != GeomAbs_Plane)
      return failure("EXTRUDE_FACE_NOT_PLANAR", "Select a planar face");
    gp_Dir direction = surface.Plane().Axis().Direction();
    if (face.Orientation() == TopAbs_REVERSED) direction.Reverse();
    const bool add_material = distance_mm > 0.0;
    if (!add_material) direction.Reverse();
    BRepFeat_MakePrism operation(solid, face, face, direction, add_material ? 1 : 0, false);
    operation.Perform(std::abs(distance_mm));
    if (!operation.IsDone())
      return failure("EXTRUDE_FACE_FAILED", "The face prism could not be built");
    auto result = checked(operation.Shape());
    if (!result->ok()) return result;
    solids.Clear();
    TopExp::MapShapes(operation.Shape(), TopAbs_SOLID, solids);
    if (solids.Extent() != 1)
      return failure("EXTRUDE_FACE_FAILED", "The result must remain one solid");
    const double volume_change = result->volume_mm3() - source.volume_mm3();
    if ((add_material ? volume_change : -volume_change) <= Precision::Confusion())
      return failure("EXTRUDE_FACE_FAILED", "The solid volume did not change in the expected direction");
    return result;
  } catch (const Standard_Failure& error) {
    return failure("EXTRUDE_FACE_FAILED", error.GetMessageString());
  } catch (...) {
    return failure("EXTRUDE_FACE_FAILED", "Unexpected face extrusion error");
  }
}

} // namespace forma
