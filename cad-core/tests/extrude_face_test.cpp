#include "forma_core.hpp"
#include "test_support.hpp"

#include <BRepCheck_Analyzer.hxx>
#include <STEPControl_Reader.hxx>
#include <chrono>
#include <cmath>
#include <filesystem>
#include <limits>

namespace {

void near(double actual, double expected, const char* message) {
  require(std::abs(actual - expected) < 1e-6, message);
}

void check_round_trip(const forma::Shape& solid, double expected_volume) {
  const auto path = std::filesystem::temp_directory_path() /
    ("forma-face-extrusion-" + std::to_string(std::chrono::steady_clock::now().time_since_epoch().count()) + ".step");
  std::string error;
  require(forma::write_step(solid, path.string(), error), "Write extrusion STEP");
  STEPControl_Reader reader;
  require(reader.ReadFile(path.string().c_str()) == IFSelect_RetDone &&
    reader.TransferRoots() > 0, "Read extrusion STEP");
  require(BRepCheck_Analyzer(reader.OneShape()).IsValid(), "STEP round-trip BREP");
  auto imported = forma::read_step(path.string());
  require(imported->ok(), "Import extruded solid");
  near(imported->volume_mm3(), expected_volume, "STEP volume");
  std::filesystem::remove(path);
}

} // namespace

int main() {
  return run_test([] {
    const auto box = forma::make_referenced_box(20, 10, 4, "pad");
    const auto added = forma::extrude_face(*box, "pad", "box-face:zmax", "", 3.);
    require(added->ok(), added->error_message().c_str());
    near(added->volume_mm3(), 1400., "Positive extrusion adds the face area times distance");
    near(added->extent_z_mm(), 7., "Positive extrusion increases height");
    check_round_trip(*added, 1400.);
    const auto removed = forma::extrude_face(*box, "pad", "box-face:zmin", "", -1.5);
    require(removed->ok(), removed->error_message().c_str());
    near(removed->volume_mm3(), 500., "Negative extrusion removes the face area times distance");
    near(removed->extent_z_mm(), 2.5, "Negative extrusion decreases height");
    check_round_trip(*removed, 500.);
    near(box->volume_mm3(), 800., "Source geometry remains unchanged");
    require(!forma::extrude_face(*box, "pad", "box-face:zmax", "missing", 2.)->ok(),
      "Reject stale occurrence");
    require(!forma::extrude_face(*box, "pad", "box-face:zmax", "", 0.)->ok(),
      "Reject zero distance");
    require(!forma::extrude_face(*box, "pad", "box-face:zmax", "",
      std::numeric_limits<double>::infinity())->ok(), "Reject infinite distance");
    require(!forma::extrude_face(*box, "pad", "box-face:zmax", "", -5.)->ok(),
      "Reject removing the whole solid");
    const auto cylinder = forma::make_referenced_cylinder(5, 10, "cylinder");
    const auto extended = forma::extrude_face(*cylinder, "cylinder", "cylinder-face:top", "", 2.);
    require(extended->ok(), extended->error_message().c_str());
    near(extended->extent_z_mm(), 12., "A cylinder cap can be extended");
    near(extended->volume_mm3(), 300. * std::acos(-1.), "Cap extrusion adds a cylindrical segment");
    require(!forma::extrude_face(*cylinder, "cylinder", "cylinder-face:side", "", 1.)->ok(),
      "Reject nonplanar face");
  });
}
