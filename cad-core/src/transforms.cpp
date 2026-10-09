#include "internal/shape_impl.hpp"

#include <BRepBuilderAPI_Transform.hxx>
#include <Standard_Failure.hxx>
#include <gp_Trsf.hxx>
#include <gp_Vec.hxx>

#include <cmath>

namespace forma {

std::unique_ptr<Shape> translate(const Shape& source, double x, double y, double z) noexcept {
  if (!source.ok()) return failure("INVALID_SOURCE", "Source shape is invalid");
  if (!std::isfinite(x) || !std::isfinite(y) || !std::isfinite(z) ||
      std::abs(x) > 10000.0 || std::abs(y) > 10000.0 || std::abs(z) > 10000.0)
    return failure("INVALID_OFFSET", "Translation is outside supported bounds");
  try {
    gp_Trsf transform;
    transform.SetTranslation(gp_Vec(x, y, z));
    BRepBuilderAPI_Transform builder(source.impl_->shape, transform, true);
    auto output = checked(builder.Shape());
    if (output->ok()) output->impl_->topology = transformed_catalog(source.impl_->topology, builder, output->impl_->shape);
    return output;
  } catch (const Standard_Failure& error) {
    return failure("OCCT_ERROR", error.GetMessageString());
  } catch (...) {
    return failure("NATIVE_ERROR", "Translation failed");
  }
}

} // namespace forma
