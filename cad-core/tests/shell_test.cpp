#include "forma_core.hpp"
#include "test_support.hpp"

#include <chrono>
#include <cmath>
#include <filesystem>

namespace {

void near(double actual, double expected, const char* message) {
  require(std::abs(actual - expected) < 1e-6, message);
}

void round_trip(const forma::Shape& solid, double expected_volume) {
  const auto path = std::filesystem::temp_directory_path() /
    ("forma-shell-" + std::to_string(std::chrono::steady_clock::now().time_since_epoch().count()) + ".step");
  std::string error;
  require(forma::write_step(solid, path.string(), error), "write shell STEP");
  const auto imported = forma::read_step(path.string());
  std::filesystem::remove(path);
  require(imported->ok(), "STEP shell BREP");
  near(imported->volume_mm3(), expected_volume, "STEP shell volume");
}

} // namespace

int main() {
  return run_test([] {
    const auto box = forma::make_referenced_box(20., 10., 4., "pad");
    const auto inward = forma::shell(*box, "pad", "box-face:zmax", "", -1.);
    require(inward->ok(), inward->error_message().c_str());
    near(inward->volume_mm3(), 368., "inward shell wall volume");
    near(inward->extent_z_mm(), 4., "inward shell keeps outside dimensions");
    round_trip(*inward, 368.);

    const auto outward = forma::shell(*box, "pad", "box-face:zmax", "", 1.);
    require(outward->ok(), outward->error_message().c_str());
    near(outward->volume_mm3(), 520., "outward shell wall volume");
    near(outward->extent_x_mm(), 22., "outward shell increases width");
    round_trip(*outward, 520.);

    const auto tube = forma::shell(*box, "pad", "box-face:zmax\nbox-face:zmin", "", -1.);
    require(tube->ok(), tube->error_message().c_str());
    near(tube->volume_mm3(), 224., "two opposing openings form a rectangular tube");
    require(!forma::shell(*box, "pad", "box-face:zmax", "wrong", -1.)->ok(),
            "stale occurrence was accepted");
    require(!forma::shell(*box, "pad", "box-face:zmax\nbox-face:zmax", "", -1.)->ok(),
            "duplicate opening was accepted");
    require(!forma::shell(*box, "pad", "box-face:zmax", "", -12.)->ok(),
            "oversized inward wall was accepted");
  });
}
