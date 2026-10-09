#include "internal/shape_impl.hpp"

#include <BRepAlgoAPI_Common.hxx>
#include <BRepAlgoAPI_Cut.hxx>
#include <BRepAlgoAPI_Fuse.hxx>
#include <Standard_Failure.hxx>

namespace forma {

enum class BooleanKind { Union, Cut, Intersect };

std::unique_ptr<Shape> boolean_apply(const Shape& left, const Shape& right, BooleanKind kind) noexcept {
  if (!left.ok() || !right.ok()) return failure("INVALID_SOURCE", "Boolean input is invalid");
  try {
    switch (kind) {
      case BooleanKind::Union: {
        BRepAlgoAPI_Fuse operation(left.impl_->shape, right.impl_->shape);
        operation.Build();
        if (!operation.IsDone()) return failure("BOOLEAN_FAILED", "Union did not complete");
        return checked(operation.Shape());
      }
      case BooleanKind::Cut: {
        BRepAlgoAPI_Cut operation(left.impl_->shape, right.impl_->shape);
        operation.Build();
        if (!operation.IsDone()) return failure("BOOLEAN_FAILED", "Cut did not complete");
        return checked(operation.Shape());
      }
      case BooleanKind::Intersect: {
        BRepAlgoAPI_Common operation(left.impl_->shape, right.impl_->shape);
        operation.Build();
        if (!operation.IsDone()) return failure("BOOLEAN_FAILED", "Intersection did not complete");
        return checked(operation.Shape());
      }
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
