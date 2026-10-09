#include "internal/shape_impl.hpp"
#include "internal/transform_axis.hpp"
#include <BRepBuilderAPI_Transform.hxx>
#include <Standard_Failure.hxx>
#include <gp_Ax1.hxx>
#include <gp_Dir.hxx>
#include <gp_Pnt.hxx>
#include <gp_Trsf.hxx>
#include <numbers>

namespace forma {
std::unique_ptr<Shape> rotate(const Shape& source, double ox, double oy, double oz,
    double dx, double dy, double dz, double angle) noexcept {
  if (!source.ok()) return failure("INVALID_SOURCE", "Source shape is invalid");
  if (!valid_transform_axis(ox, oy, oz, dx, dy, dz) || !std::isfinite(angle) || std::abs(angle) > 360)
    return failure("INVALID_TRANSFORM", "Rotation axis and angle must be finite and bounded");
  try {
    gp_Trsf transform;
    transform.SetRotation(gp_Ax1(gp_Pnt(ox, oy, oz), gp_Dir(dx, dy, dz)), angle * std::numbers::pi / 180.0);
    BRepBuilderAPI_Transform builder(source.impl_->shape, transform, true);
    auto output = checked(builder.Shape());
    if (output->ok()) output->impl_->topology = transformed_catalog(source.impl_->topology, builder, output->impl_->shape);
    return output;
  } catch (const Standard_Failure& error) {
    return failure("OCCT_ERROR", error.GetMessageString());
  } catch (...) {
    return failure("NATIVE_ERROR", "Rotation failed");
  }
}
} // namespace forma
