#include "internal/shape_impl.hpp"
#include "internal/transform_axis.hpp"

#include <BRep_Builder.hxx>
#include <BRepBuilderAPI_Transform.hxx>
#include <Standard_Failure.hxx>
#include <TopoDS_Compound.hxx>
#include <gp_Ax1.hxx>
#include <gp_Trsf.hxx>
#include <gp_Vec.hxx>
#include <cmath>
#include <numbers>

namespace forma {
namespace {

constexpr int max_instances = 128;
constexpr double min_step = 1e-7;

bool valid_step(const gp_Vec& step, int count) {
  const double length = step.Magnitude();
  return std::isfinite(length) && length <= 10000. &&
    (count == 1 || (length > min_step && length * (count - 1) <= 10000.));
}

void add_instance(const TopoDS_Shape& source, const gp_Trsf& placement,
                  BRep_Builder& builder, TopoDS_Compound& result) {
  BRepBuilderAPI_Transform copy(source, placement, true);
  builder.Add(result, copy.Shape());
}

} // namespace

std::unique_ptr<Shape> pattern_linear(const Shape& source, int count_u,
    double ux, double uy, double uz, int count_v, double vx, double vy, double vz) noexcept {
  if (!source.ok()) return failure("INVALID_SOURCE", "Pattern source is invalid");
  if (count_u < 2 || count_u > max_instances || count_v < 1 || count_v > max_instances / count_u)
    return failure("PATTERN_INVALID", "Invalid linear pattern counts");
  try {
    const gp_Vec u(ux, uy, uz), v(vx, vy, vz);
    if (!valid_step(u, count_u) || !valid_step(v, count_v) ||
        (count_v > 1 && u.Crossed(v).Magnitude() <= min_step * u.Magnitude() * v.Magnitude()))
      return failure("PATTERN_INVALID", "Pattern steps must be bounded and independent");
    BRep_Builder builder;
    TopoDS_Compound result;
    builder.MakeCompound(result);
    for (int row = 0; row < count_v; ++row) {
      for (int col = 0; col < count_u; ++col) {
        gp_Trsf placement;
        placement.SetTranslation(u * col + v * row);
        add_instance(source.impl_->shape, placement, builder, result);
      }
    }
    return checked(result);
  } catch (const Standard_Failure& error) {
    return failure("PATTERN_FAILED", error.GetMessageString());
  } catch (...) {
    return failure("PATTERN_FAILED", "Linear pattern failed");
  }
}

std::unique_ptr<Shape> pattern_circular(const Shape& source, int count, double ox, double oy,
    double oz, double dx, double dy, double dz, double step_deg) noexcept {
  if (!source.ok()) return failure("INVALID_SOURCE", "Pattern source is invalid");
  if (count < 2 || count > max_instances || !valid_transform_axis(ox, oy, oz, dx, dy, dz) ||
      !std::isfinite(step_deg) || std::abs(step_deg) <= min_step ||
      std::abs(step_deg) * (count - 1) >= 360. - min_step)
    return failure("PATTERN_INVALID", "Invalid circular pattern axis, count or angle");
  try {
    const gp_Ax1 axis(gp_Pnt(ox, oy, oz), gp_Dir(dx, dy, dz));
    const double step_rad = step_deg * std::numbers::pi / 180.;
    BRep_Builder builder;
    TopoDS_Compound result;
    builder.MakeCompound(result);
    for (int index = 0; index < count; ++index) {
      gp_Trsf placement;
      placement.SetRotation(axis, step_rad * index);
      add_instance(source.impl_->shape, placement, builder, result);
    }
    return checked(result);
  } catch (const Standard_Failure& error) {
    return failure("PATTERN_FAILED", error.GetMessageString());
  } catch (...) {
    return failure("PATTERN_FAILED", "Circular pattern failed");
  }
}

} // namespace forma
