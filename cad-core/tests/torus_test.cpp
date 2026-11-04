#include "forma_core.hpp"
#include "test_support.hpp"

#include <BRepCheck_Analyzer.hxx>
#include <STEPControl_Reader.hxx>
#include <chrono>
#include <cmath>
#include <filesystem>
#include <limits>
#include <numbers>

namespace {
constexpr double pi = std::numbers::pi_v<double>;

void check_measurements(const forma::Shape& shape) {
  require(std::abs(shape.volume_mm3() - 180 * pi * pi) < 1e-5, "Torus volume");
  require(std::abs(shape.surface_area_mm2() - 120 * pi * pi) < 1e-5, "Torus surface area");
  require(std::abs(shape.extent_x_mm() - 26) < 1e-5, "Torus X extent");
  require(std::abs(shape.extent_y_mm() - 26) < 1e-5, "Torus Y extent");
  require(std::abs(shape.extent_z_mm() - 6) < 1e-5, "Torus Z extent");
}
}

int main() {
  return run_test([] {
    const auto torus = forma::make_torus(10, 3);
    require(torus->ok(), "Valid ring torus");
    check_measurements(*torus);
    const auto path = std::filesystem::temp_directory_path() /
      ("forma-torus-" + std::to_string(std::chrono::steady_clock::now().time_since_epoch().count()) + ".step");
    std::string error;
    require(forma::write_step(*torus, path.string(), error), "Export torus to STEP");
    STEPControl_Reader reader;
    require(reader.ReadFile(path.string().c_str()) == IFSelect_RetDone &&
            reader.TransferRoots() > 0, "Read torus STEP");
    require(BRepCheck_Analyzer(reader.OneShape()).IsValid(), "Round-trip torus BREP validity");
    const auto restored = forma::read_step(path.string());
    require(restored->ok(), "Import torus STEP");
    check_measurements(*restored);
    std::filesystem::remove(path);

    require(!forma::make_torus(3, 3)->ok(), "Reject zero hole");
    require(!forma::make_torus(2, 3)->ok(), "Reject self-intersection");
    require(!forma::make_torus(10, 0)->ok(), "Reject zero tube");
    require(!forma::make_torus(10001, 3)->ok(), "Reject excessive radius");
    require(!forma::make_torus(10, std::numeric_limits<double>::quiet_NaN())->ok(), "Reject NaN");
  });
}
