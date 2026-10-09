#include "internal/shape_impl.hpp"
#include "internal/transform_axis.hpp"
#include <BRepBuilderAPI_Transform.hxx>
#include <Standard_Failure.hxx>
#include <gp_Ax2.hxx>
#include <gp_Dir.hxx>
#include <gp_Pnt.hxx>
#include <gp_Trsf.hxx>

namespace forma {
std::unique_ptr<Shape> mirror(const Shape& source, double ox, double oy, double oz,
    double nx, double ny, double nz) noexcept {
  if (!source.ok()) return failure("INVALID_SOURCE", "Source shape is invalid");
  if (!valid_transform_axis(ox, oy, oz, nx, ny, nz))
    return failure("INVALID_TRANSFORM", "Mirror plane must have a finite nonzero normal and bounded origin");
  try {
    gp_Trsf transform;
    transform.SetMirror(gp_Ax2(gp_Pnt(ox, oy, oz), gp_Dir(nx, ny, nz)));
    BRepBuilderAPI_Transform builder(source.impl_->shape, transform, true);
    auto output = checked(builder.Shape());
    if (output->ok()) output->impl_->topology = transformed_catalog(source.impl_->topology, builder, output->impl_->shape);
    return output;
  } catch (const Standard_Failure& error) {
    return failure("OCCT_ERROR", error.GetMessageString());
  } catch (...) {
    return failure("NATIVE_ERROR", "Mirror failed");
  }
}
} // namespace forma
