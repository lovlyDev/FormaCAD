#include "internal/shape_impl.hpp"

#include <BRepBuilderAPI_GTransform.hxx>
#include <BRepBuilderAPI_Transform.hxx>
#include <Standard_Failure.hxx>
#include <gp_GTrsf.hxx>
#include <gp_Mat.hxx>
#include <gp_XYZ.hxx>
#include <cmath>

namespace forma {

std::unique_ptr<Shape> transform_affine(const Shape& source, const double* matrix,
    std::size_t count) noexcept {
  if (!source.ok()) return failure("INVALID_SOURCE", "Affine transform requires a valid source solid");
  if (!matrix || count != 12)
    return failure("INVALID_TRANSFORM", "Expected a row-major 3 by 4 matrix");
  for (std::size_t i = 0; i < count; ++i) {
    const double bound = i % 4 == 3 ? 10000.0 : 1000.0;
    if (!std::isfinite(matrix[i]) || std::abs(matrix[i]) > bound)
      return failure("INVALID_TRANSFORM", "Affine transform is outside supported bounds");
  }
  try {
    const gp_Mat linear(matrix[0], matrix[1], matrix[2],
                        matrix[4], matrix[5], matrix[6],
                        matrix[8], matrix[9], matrix[10]);
    // Reflection is a separate operation. Collapsing a solid is never a scale preview commit.
    if (!std::isfinite(linear.Determinant()) || linear.Determinant() <= 1e-18)
      return failure("INVALID_TRANSFORM", "Affine transform must preserve a nonzero positive volume");
    gp_GTrsf transform(linear, gp_XYZ(matrix[3], matrix[7], matrix[11]));
    transform.SetForm();
    std::unique_ptr<BRepBuilderAPI_ModifyShape> builder;
    if (transform.Form() != gp_Other) {
      // Keep analytic planes/circles for rigid and uniform-scale transformations.
      gp_Trsf similarity;
      similarity.SetValues(matrix[0], matrix[1], matrix[2], matrix[3],
                           matrix[4], matrix[5], matrix[6], matrix[7],
                           matrix[8], matrix[9], matrix[10], matrix[11]);
      builder = std::make_unique<BRepBuilderAPI_Transform>(source.impl_->shape, similarity, true);
    } else {
      builder = std::make_unique<BRepBuilderAPI_GTransform>(source.impl_->shape, transform, true);
    }
    if (!builder->IsDone()) return failure("INVALID_TRANSFORM", "Affine transform did not complete");
    auto output = checked(builder->Shape());
    if (output->ok()) {
      output->impl_->topology = transformed_catalog(source.impl_->topology, *builder, output->impl_->shape);
      if (!source.impl_->topology.empty() && output->impl_->topology.empty())
        return failure("TOPOLOGY_REFERENCE_UNRESOLVED", "Affine transform could not preserve semantic references");
    }
    return output;
  } catch (const Standard_Failure& error) {
    return failure("OCCT_ERROR", error.GetMessageString());
  } catch (...) {
    return failure("NATIVE_ERROR", "Affine transform failed");
  }
}

} // namespace forma
