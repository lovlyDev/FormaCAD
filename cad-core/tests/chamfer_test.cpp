#include "forma_core.hpp"
#include "test_support.hpp"
#include <chrono>
#include <cmath>
#include <filesystem>

int main() {
  return run_test([] {
    auto box = forma::make_box(60.0, 40.0, 10.0);
    require(box && box->ok(), "box construction failed");
    auto chamfered = forma::chamfer_all_edges(*box, 2.0);
    require(chamfered && chamfered->ok(), "all-edge chamfer failed");
    require(chamfered->volume_mm3() < box->volume_mm3(), "chamfer did not remove material");
    require(chamfered->volume_mm3() > 0.0, "chamfer produced an empty solid");
    auto invalid = forma::chamfer_all_edges(*box, 0.0);
    require(invalid && !invalid->ok() && invalid->error_code() == "INVALID_DIMENSION",
            "zero distance was accepted");
    auto oversized = forma::chamfer_all_edges(*box, 100.0);
    require(oversized && !oversized->ok(), "oversized chamfer was accepted");

    const auto referenced = forma::make_referenced_box(60., 40., 10., "pad");
    constexpr auto edge = "box-edge:x:ymin:zmax";
    const auto selected = forma::chamfer_referenced_edge(*referenced, "pad", edge, "", 2.);
    require(selected->ok(), selected->error_message().c_str());
    require(std::abs(selected->volume_mm3() - (24000. - 60. * 2.)) < 1e-6,
            "selected chamfer removes just one triangular prism");
    const auto path = std::filesystem::temp_directory_path() /
      ("forma-selected-chamfer-" + std::to_string(std::chrono::steady_clock::now().time_since_epoch().count()) + ".step");
    std::string error;
    require(forma::write_step(*selected, path.string(), error), "write chamfered STEP");
    const auto imported = forma::read_step(path.string());
    std::filesystem::remove(path);
    require(imported->ok() && std::abs(imported->volume_mm3() - selected->volume_mm3()) < 1e-6,
            "selected chamfer BREP and volume survive STEP");
    require(!forma::chamfer_referenced_edge(*referenced, "pad", edge, "missing", 2.)->ok(),
            "stale occurrence was accepted");
  });
}
