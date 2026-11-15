#include "forma_core.hpp"
#include "test_support.hpp"

#include <cmath>

int main() {
  return run_test([] {
    auto stock = forma::make_box(60.0, 40.0, 10.0);
    auto tool = forma::make_cylinder(5.0, 10.0);
    require(stock && stock->ok() && tool && tool->ok(), "boolean input failed");
    auto result = forma::boolean_cut(*stock, *tool);
    require(result && result->ok(), "boolean cut failed");
    const double expected = 24000.0 - 250.0 * std::acos(-1.0);
    require(std::abs(result->volume_mm3() - expected) < 0.01, "cut volume mismatch");
    auto moved = forma::translate(*result, 5.0, 0.0, 0.0);
    require(moved && moved->ok(), "translation failed");
    require(std::abs(moved->volume_mm3() - expected) < 0.01, "translation changed volume");
    const auto body = forma::make_box(40., 20., 10.);
    const auto tool_box = forma::make_box(10., 10., 10.);
    const auto offset_tool = forma::translate(*tool_box, 20., 0., 0.);
    require(body->ok() && offset_tool->ok(), "Boolean pair construction");
    const auto cut = forma::boolean_cut(*body, *offset_tool);
    const auto fused = forma::boolean_union(*body, *offset_tool);
    const auto common = forma::boolean_intersect(*body, *offset_tool);
    require(cut->ok() && fused->ok() && common->ok(), "non-destructive operations failed");
    require(std::abs(cut->volume_mm3() - 7500.) < 1e-6, "cut must remove the 500 mm³ overlap");
    require(std::abs(fused->volume_mm3() - 8500.) < 1e-6, "union volume");
    require(std::abs(common->volume_mm3() - 500.) < 1e-6, "intersection volume");
    const auto far_tool = forma::translate(*offset_tool, 100., 0., 0.);
    const auto empty = forma::boolean_intersect(*body, *far_tool);
    require(!empty->ok() && empty->error_code() == "EMPTY_INTERSECTION", "disjoint intersection must explain empty result");
    const auto touching_tool = forma::translate(*tool_box, 25., 0., 0.);
    const auto touching = forma::boolean_intersect(*body, *touching_tool);
    require(!touching->ok() && touching->error_code() == "EMPTY_INTERSECTION", "face-only contact has no solid intersection");
    const auto identity = forma::boolean_intersect(*body, *body);
    require(identity->ok() && std::abs(identity->volume_mm3() - 8000.) < 1e-6, "valid common must stay valid");
    const auto reused = forma::translate(*offset_tool, 1., 0., 0.);
    require(reused->ok() && std::abs(reused->volume_mm3() - 1000.) < 1e-6, "tool no longer rebuilds");
  });
}
