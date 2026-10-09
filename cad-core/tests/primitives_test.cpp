#include "forma_core.hpp"
#include "test_support.hpp"

#include <cmath>
#include <numbers>

int main() {
  return run_test([] {
    auto box = forma::make_box(60.0, 40.0, 10.0);
    require(box && box->ok(), "box construction failed");
    require(std::abs(box->volume_mm3() - 24000.0) < 0.001, "box volume mismatch");
    require(std::abs(box->surface_area_mm2() - 6800.0) < 0.001, "box area mismatch");
    require(box->face_count() == 6 && box->edge_count() == 12, "box topology mismatch");
    require(std::abs(box->extent_x_mm() - 60.0) < 0.001 &&
                std::abs(box->extent_y_mm() - 40.0) < 0.001 &&
                std::abs(box->extent_z_mm() - 10.0) < 0.001,
            "exact box dimensions mismatch");
    auto invalid = forma::make_box(0.0, 40.0, 10.0);
    require(invalid && !invalid->ok() && invalid->error_code() == "INVALID_DIMENSION",
            "invalid dimension was accepted");
    auto sphere = forma::make_sphere(5.0);
    require(sphere && sphere->ok(), "sphere construction failed");
    require(std::abs(sphere->volume_mm3() - 4.0 * std::numbers::pi * 125.0 / 3.0) < 0.001,
            "sphere volume mismatch");
    auto cone = forma::make_cone(5.0, 0.0, 10.0);
    require(cone && cone->ok(), "cone construction failed");
    require(std::abs(cone->volume_mm3() - std::numbers::pi * 250.0 / 3.0) < 0.001,
            "cone volume mismatch");
    auto bad_cone = forma::make_cone(5.0, 5.0, 10.0);
    require(bad_cone && !bad_cone->ok(), "equal cone radii were accepted");
  });
}
