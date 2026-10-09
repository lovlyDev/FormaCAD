#include "forma_core.hpp"
#include "internal/topology_resolver.hpp"
#include <BRepPrimAPI_MakeBox.hxx>
#include <BRepClass3d_SolidClassifier.hxx>
#include <TopoDS_Face.hxx>
#include <gp_Pnt.hxx>
#include <algorithm>
#include <array>
#include <cmath>
#include <iostream>
#include <stdexcept>
#include <sstream>

namespace {
using V = std::array<double, 3>;
void require(bool condition, const char* detail) {
  if (!condition) throw std::runtime_error(detail);
}
void near(double actual, double expected) {
  if (!std::isfinite(actual) || std::abs(actual - expected) > 1e-6 * std::max(1.0, std::abs(expected))) {
    std::ostringstream detail; detail << "Analytic scalar comparison failed: " << actual << " != " << expected;
    throw std::runtime_error(detail.str());
  }
}
void near_vector(const std::vector<double>& actual, const V& expected) {
  require(actual.size() == 3, "Missing vector");
  for (std::size_t i = 0; i < 3; ++i) near(actual[i], expected[i]);
}
std::unique_ptr<forma::Shape> shape(std::unique_ptr<forma::Shape> value) {
  require(value && value->ok(), "Fixture kernel shape failed"); return value;
}
std::unique_ptr<forma::MeasurementResult> measure(const forma::Shape& source, int kind,
    const std::string& role = {}, const std::string& path = {}, const std::string& owner = "pad") {
  auto result = forma::measure_reference(source, kind, kind == 0 ? "" : owner, role, path);
  require(result && result->valid(), "Exact measurement failed"); return result;
}
void rejected(const forma::Shape& source, int kind, const std::string& owner,
    const std::string& role, const std::string& path, const std::string& code) {
  const auto result = forma::measure_reference(source, kind, owner, role, path);
  require(result && !result->valid() && result->error_code() == code, "Wrong measurement failure code");
}
V rotated(V v, double theta) { return {v[0] * std::cos(theta) - v[1] * std::sin(theta),
  v[0] * std::sin(theta) + v[1] * std::cos(theta), v[2]}; }
V mirrored(V v, V normal, V origin, bool direction) {
  double dot = 0;
  for (std::size_t i = 0; i < 3; ++i) dot += normal[i] * (v[i] - (direction ? 0 : origin[i]));
  for (std::size_t i = 0; i < 3; ++i) v[i] -= 2 * normal[i] * dot;
  return v;
}
void planes_and_transforms() {
  const double theta = 17.0 * std::acos(-1.0) / 180;
  auto base = shape(forma::make_referenced_box(40, 20, 10, "pad"));
  auto other_branch = shape(forma::make_referenced_box(40, 20, 10, "another_pad"));
  near(measure(*other_branch, 1, "box-edge:x:ymin:zmax", "", "another_pad")->length_mm(), 40);
  auto turned = shape(forma::rotate(*base, 0, 0, 0, 0, 0, 1, 17));
  turned = shape(forma::topology_occurrence(*turned, "turn"));
  auto moved = shape(forma::translate(*turned, 7, -3, 11));
  moved = shape(forma::topology_occurrence(*moved, "move"));
  const double inv = 1 / std::sqrt(3.0);
  const V mirror_normal{inv, inv, inv}, mirror_origin{2, 3, 4};
  auto flipped = shape(forma::mirror(*moved, 2, 3, 4, inv, inv, inv));
  flipped = shape(forma::topology_occurrence(*flipped, "flip"));
  const std::array<const char*, 6> roles = {"box-face:xmin", "box-face:xmax", "box-face:ymin", "box-face:ymax", "box-face:zmin", "box-face:zmax"};
  const std::array<V, 6> origins = {{{-20, 0, 5}, {20, 0, 5}, {0, -10, 5}, {0, 10, 5}, {0, 0, 0}, {0, 0, 10}}};
  const std::array<V, 6> normals = {{{-1, 0, 0}, {1, 0, 0}, {0, -1, 0}, {0, 1, 0}, {0, 0, -1}, {0, 0, 1}}};
  const std::array<double, 6> areas = {200, 200, 400, 400, 800, 800};
  for (std::size_t i = 0; i < roles.size(); ++i) {
    auto original = measure(*base, 4, roles[i]);
    near(original->area_mm2(), areas[i]); near_vector(original->origin_mm(), origins[i]); near_vector(original->normal(), normals[i]);
    auto origin = rotated(origins[i], theta), normal = rotated(normals[i], theta);
    auto rotation = measure(*turned, 4, roles[i], "turn");
    near_vector(rotation->origin_mm(), origin); near_vector(rotation->normal(), normal);
    origin[0] += 7; origin[1] -= 3; origin[2] += 11;
    auto translation = measure(*moved, 4, roles[i], "turn/move");
    near_vector(translation->origin_mm(), origin); near_vector(translation->normal(), normal);
    auto reflection = measure(*flipped, 4, roles[i], "turn/move/flip");
    near(reflection->area_mm2(), areas[i]);
    near_vector(reflection->origin_mm(), mirrored(origin, mirror_normal, mirror_origin, false));
    near_vector(reflection->normal(), mirrored(normal, mirror_normal, mirror_origin, true));
  }
  near(measure(*flipped, 1, "box-edge:x:ymin:zmax", "turn/move/flip")->length_mm(), 40);
  const auto metrics = measure(*turned, 0);
  near(metrics->volume_mm3(), 8000); near(metrics->area_mm2(), 2800);
  near_vector(metrics->extents_mm(), {40 * std::cos(theta) + 20 * std::sin(theta), 40 * std::sin(theta) + 20 * std::cos(theta), 10});
  auto resized = shape(forma::make_referenced_box(52, 20, 10, "pad"));
  near(measure(*resized, 1, "box-edge:x:ymin:zmax")->length_mm(), 52);
  near(measure(*resized, 3, "box-face:zmax")->area_mm2(), 1040);
  near(measure(*resized, 0)->volume_mm3(), 10400);
  near_vector(measure(*resized, 4, "box-face:xmin")->origin_mm(), {-26, 0, 5});
}
void invalid_and_nonmutation() {
  auto base = shape(forma::make_referenced_box(40, 20, 10, "pad"));
  rejected(*base, 1, "another_pad", "box-edge:x:ymin:zmax", "", "TOPOLOGY_REFERENCE_UNRESOLVED");
  rejected(*base, 1, "pad", "box-edge:x:ymin:zmax", "turn", "TOPOLOGY_REFERENCE_UNRESOLVED");
  rejected(*base, 1, "pad", "box-face:zmax", "", "INVALID_TOPOLOGY_REFERENCE");
  rejected(*base, 3, "pad", "box-face:zmax", "turn/turn", "INVALID_TOPOLOGY_REFERENCE");
  rejected(*base, 3, "pad", "box-face:zmax", "pad", "INVALID_TOPOLOGY_REFERENCE");
  rejected(*base, 3, "pad", "box-face:zmax", "turn/", "INVALID_TOPOLOGY_REFERENCE");
  rejected(*base, 2, "pad", "box-edge:x:ymin:zmax", "", "MEASUREMENT_UNSUPPORTED");
  rejected(*base, -1, "", "", "", "INVALID_MEASUREMENT_QUERY");
  rejected(*base, 0, "pad", "", "", "INVALID_MEASUREMENT_QUERY");
  auto plain = shape(forma::make_box(40, 20, 10));
  rejected(*plain, 3, "pad", "box-face:zmax", "", "TOPOLOGY_REFERENCE_UNSUPPORTED");
  near(measure(*plain, 0)->volume_mm3(), 8000);
  // Queries must not invalidate catalogue or modify the source before a real fillet.
  measure(*base, 1, "box-edge:x:ymin:zmax"); measure(*base, 4, "box-face:xmin");
  auto rounded = shape(forma::fillet_referenced_edge(*base, "pad", "box-edge:x:ymin:zmax", "", 1.25));
  near(rounded->volume_mm3(), 8000 - 40 * 1.25 * 1.25 * (1 - std::acos(-1.0) / 4));
  rejected(*rounded, 1, "pad", "box-edge:x:ymin:zmax", "", "TOPOLOGY_REFERENCE_UNSUPPORTED");
  near(measure(*base, 0)->volume_mm3(), 8000);
  auto overflow = shape(forma::make_referenced_box(40, 20, 10, "pad"));
  for (int i = 0; i < 65; ++i) overflow = shape(forma::topology_occurrence(*overflow, "transform_" + std::to_string(i)));
  rejected(*overflow, 3, "pad", "box-face:zmax", "", "TOPOLOGY_REFERENCE_UNSUPPORTED");
  near(measure(*overflow, 0)->volume_mm3(), 8000);
  auto invalid = forma::failure("INVALID_SOURCE", "Fixture"); require(!!invalid, "Fixture allocation failed");
  rejected(*invalid, 0, "", "", "", "INVALID_SOURCE");
  forma::Shape empty;
  rejected(empty, 0, "", "", "", "INVALID_SOURCE");
}
void resolver_and_outward_classifier() {
  BRepPrimAPI_MakeBox box(gp_Pnt(-20, -10, 0), 40, 20, 10);
  forma::TopologyCatalog entries{{box.TopFace(), "pad", "box-face:zmax", {}}};
  auto selected = forma::resolve_topology(box.Shape(), entries, TopAbs_FACE, "pad", "box-face:zmax", "");
  require(selected.valid(), "Actual constructor face should resolve");
  entries.push_back(entries[0]);
  require(forma::resolve_topology(box.Shape(), entries, TopAbs_FACE, "pad", "box-face:zmax", "").code == "TOPOLOGY_REFERENCE_AMBIGUOUS", "Ambiguity must be rejected");
  BRepPrimAPI_MakeBox other(gp_Pnt(100, 100, 100), 40, 20, 10);
  entries = {{other.TopFace(), "pad", "box-face:zmax", {}}};
  require(forma::resolve_topology(box.Shape(), entries, TopAbs_FACE, "pad", "box-face:zmax", "").code == "TOPOLOGY_REFERENCE_UNRESOLVED", "Membership must be exact");
  auto base = shape(forma::make_referenced_box(40, 20, 10, "pad"));
  auto plane = measure(*base, 4, "box-face:zmax");
  const auto& p = plane->origin_mm(); const auto& n = plane->normal();
  BRepClass3d_SolidClassifier outside(box.Shape(), gp_Pnt(p[0] + n[0] * .01, p[1] + n[1] * .01, p[2] + n[2] * .01), 1e-8);
  BRepClass3d_SolidClassifier inside(box.Shape(), gp_Pnt(p[0] - n[0] * .01, p[1] - n[1] * .01, p[2] - n[2] * .01), 1e-8);
  require(outside.State() == TopAbs_OUT && inside.State() == TopAbs_IN, "Normal must point outward");
}
}
int main() {
  try { planes_and_transforms(); invalid_and_nonmutation(); resolver_and_outward_classifier(); }
  catch (const std::exception& error) { std::cerr << error.what() << '\n'; return 1; }
  std::cout << "Reference measurements: analytic and negative checks passed\n";
  return 0;
}
