#include "internal/shape_impl.hpp"
#include <BRepPrimAPI_MakeCylinder.hxx>
#include <Standard_Failure.hxx>
#include <TopExp_Explorer.hxx>
#include <algorithm>
#include <cmath>

namespace forma {
namespace {
bool valid_owner(const std::string& owner) {
  return !owner.empty() && owner.size() <= 80 &&
    std::all_of(owner.begin(), owner.end(), [](unsigned char c) {
      return (c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z') ||
        (c >= '0' && c <= '9') || c == '-' || c == '_';
    });
}
bool member(const TopoDS_Shape& body, const TopoDS_Shape& entity) {
  if (entity.IsNull()) return false;
  for (TopExp_Explorer it(body, entity.ShapeType()); it.More(); it.Next())
    if (it.Current().IsSame(entity)) return true;
  return false;
}
}
std::unique_ptr<Shape> make_referenced_cylinder(double radius, double height,
    const std::string& owner) noexcept {
  if (!valid_owner(owner)) return failure("INVALID_TOPOLOGY_REFERENCE", "Invalid owner ID");
  if (!std::isfinite(radius) || !std::isfinite(height) || radius <= 0 || height <= 0 ||
      radius > 10000 || height > 10000)
    return failure("INVALID_DIMENSION", "Positive bounded circular extrusion required");
  try {
    BRepPrimAPI_MakeCylinder builder(radius, height);
    auto output = checked(builder.Shape());
    if (!output->ok()) return output;
    auto& cylinder = builder.Cylinder();
    // Direct constructor identities. The parametric seam is deliberately absent:
    // it is not a physical circular boundary and has no v1 stable reference.
    TopologyCatalog catalog = {
      {cylinder.BottomEdge(), owner, "cylinder-edge:bottom", {}},
      {cylinder.TopEdge(), owner, "cylinder-edge:top", {}},
      {cylinder.BottomFace(), owner, "cylinder-face:bottom", {}},
      {cylinder.TopFace(), owner, "cylinder-face:top", {}},
      {cylinder.LateralFace(), owner, "cylinder-face:side", {}}};
    for (std::size_t i = 0; i < catalog.size(); ++i) {
      if (!member(output->impl_->shape, catalog[i].entity))
        return failure("TOPOLOGY_REFERENCE_UNRESOLVED", "Cylinder constructor entity is absent");
      for (std::size_t j = 0; j < i; ++j)
        if (catalog[i].entity.IsSame(catalog[j].entity))
          return failure("TOPOLOGY_REFERENCE_AMBIGUOUS", "Cylinder constructor entity is ambiguous");
    }
    output->impl_->topology = std::move(catalog);
    return output;
  } catch (const Standard_Failure& error) { return failure("OCCT_ERROR", error.GetMessageString()); }
  catch (...) { return failure("NATIVE_ERROR", "Referenced circular extrusion failed"); }
}
}
