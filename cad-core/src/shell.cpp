#include "internal/shape_impl.hpp"
#include "internal/topology_resolver.hpp"

#include <BRepOffsetAPI_MakeThickSolid.hxx>
#include <NCollection_List.hxx>
#include <Precision.hxx>
#include <Standard_Failure.hxx>
#include <TopExp_Explorer.hxx>
#include <cmath>
#include <sstream>
#include <unordered_set>

namespace forma {

std::unique_ptr<Shape> shell(const Shape& source, const std::string& owner,
                             const std::string& roles, const std::string& occurrence,
                             double offset_mm) noexcept {
  if (!source.ok()) return failure("INVALID_SOURCE", "Shell requires a valid solid");
  if (!std::isfinite(offset_mm) || std::abs(offset_mm) <= Precision::Confusion() ||
      std::abs(offset_mm) > 10000.0)
    return failure("INVALID_DIMENSION", "Shell offset is invalid");
  if (roles.empty() || roles.size() > 1024 || roles.back() == '\n')
    return failure("INVALID_TOPOLOGY_REFERENCE", "Shell requires a bounded set of face roles");
  try {
    const auto& solid = source.impl_->shape;
    TopExp_Explorer solid_scan(solid, TopAbs_SOLID);
    if (!solid_scan.More())
      return failure("SHELL_FAILED", "Shell requires one solid");
    solid_scan.Next();
    if (solid_scan.More())
      return failure("SHELL_FAILED", "Shell requires one solid");

    NCollection_List<TopoDS_Shape> openings;
    std::unordered_set<std::string> used;
    std::istringstream entries(roles);
    for (std::string role; std::getline(entries, role);) {
      if (used.size() >= 16 || !used.insert(role).second)
        return failure("INVALID_TOPOLOGY_REFERENCE", "Shell opening roles must be unique");
      const auto face = resolve_topology(solid, source.impl_->topology,
        TopAbs_FACE, owner, role, occurrence);
      if (!face.valid()) return failure(face.code.c_str(), face.detail.c_str());
      openings.Append(face.entity);
    }
    if (openings.Size() >= source.face_count())
      return failure("SHELL_FAILED", "Shell cannot remove every face");

    BRepOffsetAPI_MakeThickSolid operation;
    operation.MakeThickSolidByJoin(solid, openings, offset_mm, Precision::Confusion(),
                                  BRepOffset_Skin, false, false, GeomAbs_Intersection);
    if (!operation.IsDone())
      return failure("SHELL_FAILED", "Shell could not be built with these openings and offset");
    auto result = checked(operation.Shape());
    if (!result->ok()) return result;
    if (offset_mm < 0.0 && result->volume_mm3() >= source.volume_mm3() - Precision::Confusion())
      return failure("SHELL_FAILED", "An inward shell must leave an open cavity inside the original solid");
    TopExp_Explorer result_scan(operation.Shape(), TopAbs_SOLID);
    if (!result_scan.More())
      return failure("SHELL_FAILED", "Shell result is not a solid");
    result_scan.Next();
    if (result_scan.More())
      return failure("SHELL_FAILED", "Shell result contains multiple solids");
    return result;
  } catch (const Standard_Failure& error) {
    return failure("SHELL_FAILED", error.GetMessageString());
  } catch (...) {
    return failure("SHELL_FAILED", "Unexpected shell error");
  }
}

} // namespace forma
