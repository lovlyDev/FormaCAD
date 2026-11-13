#include "internal/shape_impl.hpp"
#include "internal/transform_axis.hpp"
#include "forma_mirror_features.hpp"
#include <BRepAlgoAPI_Cut.hxx>
#include <BRepAlgoAPI_Fuse.hxx>
#include <NCollection_List.hxx>
#include <Standard_Failure.hxx>
#include <TopExp_Explorer.hxx>

namespace forma {
namespace {
int solid_count(const TopoDS_Shape& shape) {
  if (shape.IsNull()) return 0;
  int count = 0;
  for (TopExp_Explorer scan(shape, TopAbs_SOLID); scan.More(); scan.Next()) ++count;
  return count;
}
template<class Operation>
bool compute(const TopoDS_Shape& a, const TopoDS_Shape& b, TopoDS_Shape& result) {
  NCollection_List<TopoDS_Shape> arguments, tools;
  arguments.Append(a); tools.Append(b);
  Operation operation;
  operation.SetArguments(arguments); operation.SetTools(tools);
  operation.SetNonDestructive(true); operation.Build();
  if (!operation.IsDone()) return false;
  result = operation.Shape(); return true;
}
}
std::unique_ptr<Shape> mirror_features(const Shape& after, const Shape& before, bool has_base,
    double ox, double oy, double oz, double nx, double ny, double nz) noexcept {
  if (!after.ok() || (has_base && !before.ok()))
    return failure("INVALID_SOURCE", "Feature mirror source is invalid");
  if (!valid_transform_axis(ox, oy, oz, nx, ny, nz))
    return failure("INVALID_FEATURE_MIRROR_PLANE", "Mirror plane is invalid");
  try {
    const auto& target = after.impl_->shape;
    if (solid_count(target) != 1 || (has_base && solid_count(before.impl_->shape) != 1))
      return failure("MIRROR_FEATURES_REQUIRES_SOLID", "Feature mirror requires one solid before and after the range");
    TopoDS_Shape added, removed;
    if (has_base) {
      if (!compute<BRepAlgoAPI_Cut>(target, before.impl_->shape, added)
          || !compute<BRepAlgoAPI_Cut>(before.impl_->shape, target, removed))
        return failure("MIRROR_FEATURES_FAILED", "Could not determine material changes");
    } else added = target;
    if (!solid_count(added) && !solid_count(removed))
      return failure("MIRROR_FEATURES_NO_EFFECT", "The selected operations do not change material");
    TopoDS_Shape result = target;
    // Removed and added regions are disjoint. Work on independent reflected tools.
    for (const bool add : {false, true}) {
      const auto& delta = add ? added : removed;
      if (!solid_count(delta)) continue;
      const auto part = checked(delta);
      if (!part->ok()) return failure("MIRROR_FEATURES_FAILED", part->error_message().c_str());
      const auto reflected = mirror(*part, ox, oy, oz, nx, ny, nz);
      if (!reflected->ok()) return failure("MIRROR_FEATURES_FAILED", reflected->error_message().c_str());
      TopoDS_Shape next;
      if (add && !solid_count(result)) next = reflected->impl_->shape;
      else if (!(add ? compute<BRepAlgoAPI_Fuse>(result, reflected->impl_->shape, next)
                    : compute<BRepAlgoAPI_Cut>(result, reflected->impl_->shape, next)))
        return failure("MIRROR_FEATURES_FAILED", "Mirrored material change failed");
      result = next;
    }
    if (solid_count(result) != 1)
      return failure("MIRROR_FEATURES_DISCONNECTED", "Mirrored changes must leave one connected solid");
    auto output = checked(result);
    if (!output->ok()) return failure("MIRROR_FEATURES_FAILED", output->error_message().c_str());
    TopoDS_Shape newly_added, newly_removed;
    if (!compute<BRepAlgoAPI_Cut>(result, target, newly_added)
        || !compute<BRepAlgoAPI_Cut>(target, result, newly_removed))
      return failure("MIRROR_FEATURES_FAILED", "Could not verify the mirrored change");
    if (!solid_count(newly_added) && !solid_count(newly_removed))
      return failure("MIRROR_FEATURES_NO_EFFECT", "Mirrored changes do not alter this body");
    return output;
  } catch (const Standard_Failure& error) {
    return failure("MIRROR_FEATURES_FAILED", error.GetMessageString());
  } catch (...) {
    return failure("MIRROR_FEATURES_FAILED", "Unexpected feature mirroring error");
  }
}
}
