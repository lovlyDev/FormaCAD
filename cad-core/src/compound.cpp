#include "internal/shape_impl.hpp"

#include <BRep_Builder.hxx>
#include <Standard_Failure.hxx>
#include <TopoDS_Compound.hxx>

namespace forma {

std::unique_ptr<Shape> make_compound(const Shape& left, const Shape& right) noexcept {
  if (!left.ok() || !right.ok())
    return failure("INVALID_SOURCE", "Compound requires two valid shapes");
  try {
    BRep_Builder builder;
    TopoDS_Compound compound;
    builder.MakeCompound(compound);
    builder.Add(compound, left.impl_->shape);
    builder.Add(compound, right.impl_->shape);
    return checked(compound);
  } catch (const Standard_Failure& error) {
    return failure("OCCT_ERROR", error.GetMessageString());
  } catch (...) {
    return failure("NATIVE_ERROR", "Compound creation failed");
  }
}

} // namespace forma
