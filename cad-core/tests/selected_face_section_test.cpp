// Independent analytic oracle: authored face plane intersects whole committed STEP.
#include "forma_core.hpp"
#include <algorithm>
#include <array>
#include <chrono>
#include <cmath>
#include <filesystem>
#include <iostream>
#include <numeric>
#include <stdexcept>

namespace {
using V = std::array<double, 3>;
void require(bool condition, const char* detail) {
  if (!condition) throw std::runtime_error(detail);
}
void near(double actual, double expected) {
  require(std::isfinite(actual) && std::abs(actual - expected) <= 1e-6 * std::max(1.0, std::abs(expected)),
    "Independent face-section analytic comparison failed");
}
std::unique_ptr<forma::Shape> shape(std::unique_ptr<forma::Shape> value) {
  require(value && value->ok(), "Invalid fixture BREP"); return value;
}
std::unique_ptr<forma::MeasurementResult> plane(const forma::Shape& body, const std::string& role,
    const std::string& occurrence = "") {
  auto value = forma::measure_reference(body, 4, "pad", role, occurrence);
  require(value && value->valid(), "Exact authored planar-face resolution failed"); return value;
}
std::unique_ptr<forma::SectionResult> cut(const forma::Shape& whole_source,
    const forma::MeasurementResult& face, double offset) {
  const auto& center = face.origin_mm(); const auto& normal = face.normal();
  // Worker composition under design: positive offset is outward along the
  // actual signed CAD normal. The renderer must not supply these vectors.
  return forma::section_plane(whole_source,
    center[0] + offset * normal[0], center[1] + offset * normal[1], center[2] + offset * normal[2],
    normal[0], normal[1], normal[2], 0.01);
}
void curves(const forma::SectionResult& result, double expected_total) {
  require(result.valid(), "Exact STEP plane section failed");
  near(std::accumulate(result.edge_lengths_mm().begin(), result.edge_lengths_mm().end(), 0.0), expected_total);
  require(result.edge_offsets().size() == result.edge_lengths_mm().size() + 1 &&
    result.edge_offsets().front() == 0 && result.points_mm().size() % 3 == 0 &&
    result.edge_offsets().back() == result.points_mm().size() / 3, "Malformed bounded section curves");
}
V transform(V point, bool direction) {
  const double theta = 31 * std::acos(-1.0) / 180, c = std::cos(theta), s = std::sin(theta);
  V rotated{c * point[0] + s * point[2], point[1], -s * point[0] + c * point[2]};
  if (!direction) { rotated[0] += 3; rotated[1] -= 4; rotated[2] += 9; }
  rotated[0] = (direction ? 0 : 14) - rotated[0]; // Mirror X=7, polar vector sign.
  return rotated;
}
std::unique_ptr<forma::Shape> transformed(const forma::Shape& source, bool track) {
  auto value = shape(forma::rotate(source, 0, 0, 0, 0, 1, 0, 31));
  if (track) value = shape(forma::topology_occurrence(*value, "turn"));
  value = shape(forma::translate(*value, 3, -4, 9));
  if (track) value = shape(forma::topology_occurrence(*value, "move"));
  value = shape(forma::mirror(*value, 7, 0, 0, 1, 0, 0));
  if (track) value = shape(forma::topology_occurrence(*value, "flip"));
  return value;
}
void vector_near(const std::vector<double>& actual, V expected) {
  require(actual.size() == 3, "Missing resolved vector");
  for (int i = 0; i < 3; ++i) near(actual[i], expected[i]);
}
struct StepFixture {
  std::filesystem::path directory;
  StepFixture() {
    directory = std::filesystem::temp_directory_path() /
      ("forma-face-section-draft-" + std::to_string(std::chrono::steady_clock::now().time_since_epoch().count()));
    require(std::filesystem::create_directory(directory), "Private STEP fixture directory failed");
  }
  ~StepFixture() {
    std::error_code ignored;
    std::filesystem::remove(directory / "whole.step", ignored);
    std::filesystem::remove(directory, ignored); // Nonrecursive, owned paths only.
  }
  std::unique_ptr<forma::Shape> roundtrip(const forma::Shape& value) {
    std::string code;
    require(forma::write_step(value, (directory / "whole.step").string(), code), "Private source STEP export failed");
    return shape(forma::read_step((directory / "whole.step").string()));
  }
};
void run() {
  const double pi = std::acos(-1.0);
  auto box = shape(forma::make_referenced_box(40, 20, 10, "pad"));
  auto cylinder = shape(forma::make_cylinder(8, 15));
  cylinder = shape(forma::translate(*cylinder, 100, 0, 0));
  auto all = shape(forma::make_compound(*box, *cylinder));
  StepFixture fixture;
  auto sealed_step = fixture.roundtrip(*all);
  const double before_volume = sealed_step->volume_mm3();
  auto top = plane(*box, "box-face:zmax"), bottom = plane(*box, "box-face:zmin");
  vector_near(top->origin_mm(), {0, 0, 10}); vector_near(top->normal(), {0, 0, 1});
  vector_near(bottom->origin_mm(), {0, 0, 0}); vector_near(bottom->normal(), {0, 0, -1});
  curves(*cut(*sealed_step, *top, -5), 120 + 16 * pi);
  curves(*cut(*sealed_step, *bottom, -5), 120 + 16 * pi); // Same plane, opposite normals.
  curves(*cut(*sealed_step, *top, 6), 0); // z=16, outside BOTH committed bodies.
  curves(*cut(*sealed_step, *bottom, 5), 0); // z=-5, outward below both bodies.
  auto empty = cut(*sealed_step, *top, 6);
  require(empty->points_mm().empty() && empty->edge_lengths_mm().empty() &&
    empty->edge_offsets() == std::vector<std::uint32_t>{0}, "Empty intersection is not valid zero curves");
  // Side normal and non-origin plane: xmin(-20), outward normal(-1,0,0), offset -5 => x=-15.
  auto side = plane(*box, "box-face:xmin");
  curves(*cut(*sealed_step, *side, -5), 60);
  // Entire committed STEP, including unrelated bodies, is transformed with the
  // same independent transform oracle. Arc/line total lengths are invariant.
  auto box_moved = transformed(*box, true), all_moved = transformed(*all, false);
  auto moved_step = fixture.roundtrip(*all_moved);
  auto moved_top = plane(*box_moved, "box-face:zmax", "turn/move/flip");
  vector_near(moved_top->origin_mm(), transform({0, 0, 10}, false));
  vector_near(moved_top->normal(), transform({0, 0, 1}, true));
  curves(*cut(*moved_step, *moved_top, -5), 120 + 16 * pi);
  curves(*cut(*moved_step, *moved_top, 6), 0);
  auto circle = shape(forma::make_referenced_cylinder(8, 10, "pad"));
  auto unrelated_box = shape(forma::make_box(20, 12, 15));
  unrelated_box = shape(forma::translate(*unrelated_box, 100, 0, 0));
  auto circle_all = shape(forma::make_compound(*circle, *unrelated_box));
  auto circle_step = fixture.roundtrip(*circle_all);
  auto cap = plane(*circle, "cylinder-face:top");
  curves(*cut(*circle_step, *cap, -5), 16 * pi + 64);
  auto curved = forma::measure_reference(*circle, 4, "pad", "cylinder-face:side", "");
  require(curved && !curved->valid() && curved->error_code() == "MEASUREMENT_UNSUPPORTED", "Curved side used as fallback plane");
  auto bounded_out = cut(*sealed_step, *top, 10000);
  require(bounded_out && !bounded_out->valid() && bounded_out->error_code() == "INVALID_SECTION_PLANE", "Derived world-origin bounds ignored");
  near(sealed_step->volume_mm3(), before_volume);
  near(box->volume_mm3(), 8000);
}
}
int main() {
  try { run(); std::cout << "Selected-face section independent analytics passed\n"; return 0; }
  catch (const std::exception& error) { std::cerr << error.what() << '\n'; return 1; }
}
