#include "internal/shape_impl.hpp"
#include "forma_split.hpp"
#include <BRepAlgoAPI_Common.hxx>
#include <BRepBuilderAPI_MakeFace.hxx>
#include <BRepPrimAPI_MakeHalfSpace.hxx>
#include <NCollection_List.hxx>
#include <TopExp_Explorer.hxx>
#include <gp_Pln.hxx>
#include <gp_Vec.hxx>
#include <Standard_Failure.hxx>
#include <array>
#include <algorithm>
#include <cmath>
#include <utility>

namespace forma {
std::unique_ptr<Shape> split_by_plane(const Shape& source,
    double x, double y, double z, double nx, double ny, double nz, bool positive) noexcept {
  if (!source.ok()) return failure("INVALID_SOURCE", "Split source is invalid");
  const std::array<double, 6> values{x, y, z, nx, ny, nz};
  if (!std::all_of(values.begin(), values.end(), [](double v) { return std::isfinite(v); })
      || std::max({std::abs(x), std::abs(y), std::abs(z)}) > 10000.)
    return failure("INVALID_SPLIT_PLANE", "Split plane must have finite coordinates");
  const double norm = std::hypot(nx, ny, nz);
  if (!std::isfinite(norm) || norm < 1e-12)
    return failure("INVALID_SPLIT_PLANE", "Split plane normal must be nonzero");
  try {
    const gp_Pnt origin(x, y, z);
    const gp_Dir normal(nx / norm, ny / norm, nz / norm);
    const TopoDS_Face face = BRepBuilderAPI_MakeFace(gp_Pln(origin, normal)).Face();
    std::array<std::unique_ptr<Shape>, 2> parts;
    for (int i = 0; i < 2; ++i) {
      const gp_Pnt inside = origin.Translated(gp_Vec(normal) * (i == 0 ? 1. : -1.));
      const TopoDS_Solid halfspace = BRepPrimAPI_MakeHalfSpace(face, inside).Solid();
      NCollection_List<TopoDS_Shape> arguments, tools;
      arguments.Append(source.impl_->shape);
      tools.Append(halfspace);
      BRepAlgoAPI_Common operation;
      operation.SetArguments(arguments);
      operation.SetTools(tools);
      operation.SetNonDestructive(true);
      operation.Build();
      if (!operation.IsDone()) return failure("SPLIT_FAILED", "Plane split did not complete");
      if (operation.Shape().IsNull() || !TopExp_Explorer(operation.Shape(), TopAbs_SOLID).More())
        return failure("SPLIT_NO_INTERSECTION", "Plane does not divide the body into two solid parts");
      parts[i] = checked(operation.Shape());
      if (!parts[i] || !parts[i]->ok()) return std::move(parts[i]);
    }
    const double sum = parts[0]->volume_mm3() + parts[1]->volume_mm3();
    if (std::abs(sum - source.volume_mm3()) > std::max(1e-6, source.volume_mm3() * 1e-7))
      return failure("SPLIT_VOLUME_MISMATCH", "Split parts do not preserve source volume");
    return std::move(parts[positive ? 0 : 1]);
  } catch (const Standard_Failure& error) {
    return failure("OCCT_ERROR", error.GetMessageString());
  } catch (...) {
    return failure("NATIVE_ERROR", "Plane split failed");
  }
}
}
