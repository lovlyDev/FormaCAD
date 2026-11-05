#include "forma_core.hpp"
#include "test_support.hpp"

#include <chrono>
#include <cmath>
#include <filesystem>

namespace {

void near(double actual, double expected, const char* message) {
  require(std::abs(actual - expected) < 1e-6, message);
}

void round_trip(const forma::Shape& result, double volume) {
  const auto path = std::filesystem::temp_directory_path() /
    ("forma-pattern-" + std::to_string(std::chrono::steady_clock::now().time_since_epoch().count()) + ".step");
  std::string error;
  require(forma::write_step(result, path.string(), error), "write pattern STEP");
  const auto imported = forma::read_step(path.string());
  std::filesystem::remove(path);
  require(imported->ok(), "import valid patterned BREP");
  near(imported->volume_mm3(), volume, "STEP pattern volume");
}

} // namespace

int main() {
  return run_test([] {
    const auto source = forma::make_box(2., 2., 3.);
    const auto grid = forma::pattern_linear(*source, 3, 5., 0., 0., 2, 0., 6., 0.);
    require(grid->ok(), grid->error_message().c_str());
    near(grid->extent_x_mm(), 12., "linear U extent");
    near(grid->extent_y_mm(), 8., "linear V extent");
    near(grid->volume_mm3(), 72., "linear grid volume");
    round_trip(*grid, 72.);

    const auto displaced = forma::translate(*source, 10., 0., 0.);
    const auto circle = forma::pattern_circular(*displaced, 4, 0., 0., 0., 0., 0., 1., -90.);
    require(circle->ok(), circle->error_message().c_str());
    near(circle->extent_x_mm(), 22., "circular pattern X extent");
    near(circle->extent_y_mm(), 22., "circular pattern Y extent");
    near(circle->volume_mm3(), 48., "circular pattern volume");
    round_trip(*circle, 48.);

    require(!forma::pattern_linear(*source, 4, 0., 0., 0., 1, 0., 0., 0.)->ok(), "reject zero spacing");
    require(!forma::pattern_linear(*source, 2, 1., 0., 0., 2, 2., 0., 0.)->ok(), "reject parallel axes");
    require(!forma::pattern_linear(*source, 16, 3., 0., 0., 16, 0., 3., 0.)->ok(), "bound instance count");
    require(!forma::pattern_circular(*displaced, 5, 0., 0., 0., 0., 0., 1., 90.)->ok(), "reject repeated angle");
  });
}
