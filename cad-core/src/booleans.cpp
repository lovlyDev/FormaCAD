#include "internal/shape_impl.hpp"

#include <BRepAlgoAPI_Common.hxx>
#include <BRepAlgoAPI_Cut.hxx>
#include <BRepAlgoAPI_Fuse.hxx>
#include <NCollection_List.hxx>
#include <Standard_Failure.hxx>

namespace forma {

enum class BooleanKind { Union, Cut, Intersect };

namespace {
template<class Operation>
std::unique_ptr<Shape> apply(const TopoDS_Shape& left, const TopoDS_Shape& right, const char* detail) {
  NCollection_List<TopoDS_Shape> arguments;
  NCollection_List<TopoDS_Shape> tools;
  arguments.Append(left);
  tools.Append(right);
  Operation operation;
  operation.SetArguments(arguments);
  operation.SetTools(tools);
  operation.SetNonDestructive(true);
  operation.Build();
  if (!operation.IsDone()) return failure("BOOLEAN_FAILED", detail);
  return checked(operation.Shape());
}
}

std::unique_ptr<Shape> boolean_apply(const Shape& left, const Shape& right, BooleanKind kind) noexcept {
  if (!left.ok() || !right.ok()) return failure("INVALID_SOURCE", "Boolean input is invalid");
  try {
    switch (kind) {
      case BooleanKind::Union:
        return apply<BRepAlgoAPI_Fuse>(left.impl_->shape, right.impl_->shape, "Union did not complete");
      case BooleanKind::Cut:
        return apply<BRepAlgoAPI_Cut>(left.impl_->shape, right.impl_->shape, "Cut did not complete");
      case BooleanKind::Intersect:
        return apply<BRepAlgoAPI_Common>(left.impl_->shape, right.impl_->shape, "Intersection did not complete");
    }
  } catch (const Standard_Failure& error) {
    return failure("OCCT_ERROR", error.GetMessageString());
  } catch (...) {
    return failure("NATIVE_ERROR", "Boolean operation failed");
  }
  return failure("INVALID_OPERATION", "Unknown Boolean operation");
}

std::unique_ptr<Shape> boolean_union(const Shape& left, const Shape& right) noexcept {
  return boolean_apply(left, right, BooleanKind::Union);
}
std::unique_ptr<Shape> boolean_cut(const Shape& left, const Shape& right) noexcept {
  return boolean_apply(left, right, BooleanKind::Cut);
}
std::unique_ptr<Shape> boolean_intersect(const Shape& left, const Shape& right) noexcept {
  return boolean_apply(left, right, BooleanKind::Intersect);
}

} // namespace forma
