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
  });
}
