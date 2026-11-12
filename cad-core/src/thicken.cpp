#include "internal/shape_impl.hpp"
#include "internal/topology_resolver.hpp"
#include "forma_thicken.hpp"
#include <BRepBuilderAPI_Copy.hxx>
#include <BRepOffsetAPI_MakeThickSolid.hxx>
#include <Precision.hxx>
#include <BRepLib.hxx>
#include <TopoDS.hxx>
#include <TopoDS_Solid.hxx>
#include <Standard_Failure.hxx>
#include <TopExp_Explorer.hxx>
#include <cmath>

namespace forma {
std::unique_ptr<Shape> thicken_face(const Shape& source, const std::string& owner,
    const std::string& role, const std::string& occurrence, double offset_mm) noexcept {
  if (!source.ok()) return failure("INVALID_SOURCE", "Thicken requires a valid source");
  if (!std::isfinite(offset_mm) || std::abs(offset_mm) <= Precision::Confusion()
      || std::abs(offset_mm) > 10000.)
    return failure("INVALID_THICKNESS", "Thickness must be finite and nonzero");
  try {
    const auto reference = resolve_topology(source.impl_->shape, source.impl_->topology,
        TopAbs_FACE, owner, role, occurrence);
    if (!reference.valid()) return failure(reference.code.c_str(), reference.detail.c_str());
    // Work on an independent face; the original solid and its topology remain untouched.
    BRepBuilderAPI_Copy copy(reference.entity, true, false);
    BRepOffsetAPI_MakeThickSolid operation;
    operation.MakeThickSolidBySimple(copy.Shape(), offset_mm);
    if (!operation.IsDone())
      return failure("THICKEN_FAILED", "The selected face could not be thickened");
    const auto shape = operation.Shape();
    TopExp_Explorer solids(shape, TopAbs_SOLID);
    if (!solids.More()) return failure("THICKEN_FAILED", "Thicken must produce one solid");
    auto solid = TopoDS::Solid(solids.Current());
    solids.Next();
    if (solids.More()) return failure("THICKEN_FAILED", "Thicken produced several solids");
    if (!BRepLib::OrientClosedSolid(solid))
      return failure("THICKEN_FAILED", "Thicken result is not a closed solid");
    auto result = checked(solid);
    if (!result->ok()) return failure("THICKEN_FAILED", result->error_message().c_str());
    return result;
  } catch (const Standard_Failure& error) {
    return failure("THICKEN_FAILED", error.GetMessageString());
  } catch (...) {
    return failure("THICKEN_FAILED", "Unexpected face thickening error");
  }
}
}
