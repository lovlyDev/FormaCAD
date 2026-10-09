#include "forma_core.hpp"
#include "test_support.hpp"

#include <cmath>
#include <filesystem>
#include <string>

int main() {
  return run_test([] {
    auto shape = forma::make_box(60.0, 40.0, 10.0);
    require(shape && shape->ok(), "STEP test shape failed");
    const auto path = std::filesystem::temp_directory_path() / "forma-cad-core-roundtrip.step";
    std::string error;
    require(forma::write_step(*shape, path.string(), error), "STEP export failed");
    auto imported = forma::read_step(path.string());
    require(imported && imported->ok(), "STEP import failed");
    require(std::abs(imported->volume_mm3() - shape->volume_mm3()) < 0.01,
            "STEP round-trip volume mismatch");
    std::filesystem::remove(path);
  });
}
