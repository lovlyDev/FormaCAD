#include "forma_datum.hpp"

#include <BRepBuilderAPI_MakeEdge.hxx>
#include <BRepBuilderAPI_MakeFace.hxx>
#include <BRepBuilderAPI_MakeVertex.hxx>
#include <BRepCheck_Analyzer.hxx>
#include <Precision.hxx>
#include <TopoDS_Edge.hxx>
#include <TopoDS_Face.hxx>
#include <TopoDS_Vertex.hxx>
#include <gp_Pln.hxx>
#include <gp_Vec.hxx>
#include <algorithm>
#include <cmath>

namespace forma::datum {
namespace {

constexpr double max_dimension_mm = 10000.0;

bool bounded(const Coordinates& values, double maximum) {
  return std::all_of(values.begin(), values.end(), [maximum](double value) {
    return std::isfinite(value) && std::abs(value) <= maximum;
  });
}

bool usable_direction(const Coordinates& direction) {
  return bounded(direction, 1e6) &&
         std::hypot(direction[0], direction[1], direction[2]) > Precision::Confusion();
}

bool usable_span(double span) {
  return std::isfinite(span) && span > Precision::Confusion() && span <= max_dimension_mm;
}

gp_Pnt position(const Coordinates& origin) {
  return {origin[0], origin[1], origin[2]};
}

gp_Dir unit_direction(const Coordinates& direction) {
  return {direction[0], direction[1], direction[2]};
}

TopoDS_Shape checked(const TopoDS_Shape& shape) {
  if (shape.IsNull() || !BRepCheck_Analyzer(shape).IsValid()) return {};
  return shape;
}

} // namespace

TopoDS_Shape plane(const Coordinates& origin_mm, const Coordinates& normal,
                   double side_mm) noexcept {
  if (!bounded(origin_mm, max_dimension_mm) || !usable_direction(normal) || !usable_span(side_mm))
    return {};
  try {
    const gp_Pln support(position(origin_mm), unit_direction(normal));
    const double half = side_mm / 2.0;
    return checked(BRepBuilderAPI_MakeFace(support, -half, half, -half, half).Face());
  } catch (...) {
    return {};
  }
}

TopoDS_Shape axis(const Coordinates& origin_mm, const Coordinates& direction,
                  double length_mm) noexcept {
  if (!bounded(origin_mm, max_dimension_mm) || !usable_direction(direction) || !usable_span(length_mm))
    return {};
  try {
    const gp_Pnt origin = position(origin_mm);
    const gp_Vec half = gp_Vec(unit_direction(direction)).Multiplied(length_mm / 2.0);
    return checked(BRepBuilderAPI_MakeEdge(origin.Translated(-half), origin.Translated(half)).Edge());
  } catch (...) {
    return {};
  }
}

TopoDS_Shape point(const Coordinates& origin_mm) noexcept {
  if (!bounded(origin_mm, max_dimension_mm)) return {};
  try {
    return checked(BRepBuilderAPI_MakeVertex(position(origin_mm)).Vertex());
  } catch (...) {
    return {};
  }
}

} // namespace forma::datum
