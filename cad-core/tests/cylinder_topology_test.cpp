#include "forma_core.hpp"
#include "internal/topology_catalog.hpp"
#include "internal/topology_resolver.hpp"
#include <BRepPrimAPI_MakeCylinder.hxx>
#include <TopExp_Explorer.hxx>
#include <cmath>
#include <iostream>
#include <stdexcept>

namespace {
void require(bool condition, const char* detail) {
  if (!condition) throw std::runtime_error(detail);
}
void near(double actual, double expected) {
  require(std::isfinite(actual) && std::abs(actual - expected) < 1e-6 * std::max(1.0, std::abs(expected)),
    "Cylinder analytic comparison failed");
}
std::unique_ptr<forma::Shape> shape(std::unique_ptr<forma::Shape> value) {
  require(value && value->ok(), "Cylinder fixture failed"); return value;
}
std::unique_ptr<forma::MeasurementResult> measure(const forma::Shape& value, int kind,
    const std::string& role, const std::string& path = {}) {
  auto result = forma::measure_reference(value, kind, kind == 0 ? "" : "pad", role, path);
  require(result && result->valid(), "Cylinder measurement failed"); return result;
}
void rejected(const forma::Shape& value, int kind, const std::string& owner,
    const std::string& role, const std::string& path, const char* code) {
  auto result = forma::measure_reference(value, kind, owner, role, path);
  require(result && !result->valid() && result->error_code() == code, "Wrong cylinder rejection");
}
void check_cylinder(const forma::Shape& value, double radius, double height, const std::string& path) {
  const double pi = std::acos(-1.0);
  for (const auto* role : {"cylinder-edge:bottom", "cylinder-edge:top"}) {
    near(measure(value, 1, role, path)->length_mm(), 2 * pi * radius);
    near(measure(value, 2, role, path)->radius_mm(), radius);
    near(measure(value, 5, role, path)->diameter_mm(), 2 * radius);
  }
  for (const auto* role : {"cylinder-face:bottom", "cylinder-face:top"})
    near(measure(value, 4, role, path)->area_mm2(), pi * radius * radius);
  near(measure(value, 3, "cylinder-face:side", path)->area_mm2(), 2 * pi * radius * height);
  near(measure(value, 0, "")->volume_mm3(), pi * radius * radius * height);
  near(measure(value, 0, "")->area_mm2(), 2 * pi * radius * (radius + height));
}
void run() {
  auto base = shape(forma::make_referenced_cylinder(8, 25, "pad"));
  check_cylinder(*base, 8, 25, "");
  auto bottom = measure(*base, 4, "cylinder-face:bottom");
  auto top = measure(*base, 4, "cylinder-face:top");
  near(bottom->origin_mm()[2], 0); near(bottom->normal()[2], -1);
  near(top->origin_mm()[2], 25); near(top->normal()[2], 1);
  // Rotate the extrusion axis, then translate and reflect through an oblique
  // non-origin plane. Cap normals must remain signed outward polar vectors.
  auto rotated = shape(forma::rotate(*base, 0, 0, 0, 1, 0, 0, 90));
  rotated = shape(forma::topology_occurrence(*rotated, "turn"));
  auto moved = shape(forma::translate(*rotated, 7, -3, 11));
  moved = shape(forma::topology_occurrence(*moved, "move"));
  const double inv = 1 / std::sqrt(3.0);
  auto mirrored = shape(forma::mirror(*moved, 2, 3, 4, inv, inv, inv));
  mirrored = shape(forma::topology_occurrence(*mirrored, "flip"));
  check_cylinder(*mirrored, 8, 25, "turn/move/flip");
  for (int is_top = 0; is_top <= 1; ++is_top) {
    const auto result = measure(*mirrored, 4, is_top ? "cylinder-face:top" : "cylinder-face:bottom", "turn/move/flip");
    const double sign = is_top ? 1 : -1;
    near(result->normal()[0], sign * 2.0 / 3.0);
    near(result->normal()[1], -sign / 3.0);
    near(result->normal()[2], sign * 2.0 / 3.0);
    const double origin[3] = {7, -3 - (is_top ? 25 : 0), 11};
    const double dot = (origin[0] - 2 + origin[1] - 3 + origin[2] - 4) / 3;
    for (int i = 0; i < 3; ++i) near(result->origin_mm()[i], origin[i] - 2 * dot);
  }
  auto resized = shape(forma::make_referenced_cylinder(13, 7, "pad"));
  check_cylinder(*resized, 13, 7, "");
  rejected(*base, 4, "pad", "cylinder-face:side", "", "MEASUREMENT_UNSUPPORTED");
  rejected(*base, 5, "foreign", "cylinder-edge:top", "", "TOPOLOGY_REFERENCE_UNRESOLVED");
  rejected(*base, 2, "pad", "cylinder-edge:bottom", "turn", "TOPOLOGY_REFERENCE_UNRESOLVED");
  rejected(*base, 2, "pad", "cylinder-face:top", "", "INVALID_TOPOLOGY_REFERENCE");
  rejected(*base, 5, "pad", "cylinder-edge:seam", "", "INVALID_TOPOLOGY_REFERENCE");
  auto box = shape(forma::make_referenced_box(40, 20, 10, "pad"));
  rejected(*box, 5, "pad", "box-edge:x:ymin:zmin", "", "MEASUREMENT_UNSUPPORTED");
  auto plain = shape(forma::make_cylinder(8, 25));
  rejected(*plain, 2, "pad", "cylinder-edge:top", "", "TOPOLOGY_REFERENCE_UNSUPPORTED");
  auto united = shape(forma::boolean_union(*base, *box));
  rejected(*united, 2, "pad", "cylinder-edge:top", "", "TOPOLOGY_REFERENCE_UNSUPPORTED");
  auto bounded = shape(forma::make_referenced_cylinder(8, 25, "pad"));
  for (int i = 0; i < 65; ++i) bounded = shape(forma::topology_occurrence(*bounded, "move_" + std::to_string(i)));
  rejected(*bounded, 2, "pad", "cylinder-edge:top", "", "TOPOLOGY_REFERENCE_UNSUPPORTED");
  // Resolver guards against duplicated and foreign constructor entities.
  BRepPrimAPI_MakeCylinder builder(8, 25), foreign(8, 25);
  const auto body = builder.Shape();
  forma::TopologyCatalog catalog = {{builder.Cylinder().TopEdge(), "pad", "cylinder-edge:top", {}}};
  require(forma::resolve_topology(body, catalog, TopAbs_EDGE, "pad", "cylinder-edge:top", "").valid(), "Missing member");
  catalog.push_back(catalog.front());
  require(forma::resolve_topology(body, catalog, TopAbs_EDGE, "pad", "cylinder-edge:top", "").code == "TOPOLOGY_REFERENCE_AMBIGUOUS", "Ambiguity not rejected");
  catalog = {{foreign.Cylinder().TopEdge(), "pad", "cylinder-edge:top", {}}};
  require(forma::resolve_topology(body, catalog, TopAbs_EDGE, "pad", "cylinder-edge:top", "").code == "TOPOLOGY_REFERENCE_UNRESOLVED", "Foreign BREP accepted");
  for (double dimension : {0.0, -1.0, 10001.0})
    require(!forma::make_referenced_cylinder(dimension, 25, "pad")->ok() &&
      !forma::make_referenced_cylinder(8, dimension, "pad")->ok(), "Invalid dimension accepted");
  require(!forma::make_referenced_cylinder(8, 25, "bad/owner")->ok(), "Invalid owner accepted");
  check_cylinder(*base, 8, 25, ""); // Read-only queries preserve original provenance.
}
}
int main() {
  try { run(); std::cout << "Cylinder provenance and exact measurements passed\n"; return 0; }
  catch (const std::exception& error) { std::cerr << error.what() << '\n'; return 1; }
}
