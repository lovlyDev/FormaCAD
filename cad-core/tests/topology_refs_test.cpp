#include "forma_core.hpp"
#include <STEPControl_Reader.hxx>
#include <IFSelect_ReturnStatus.hxx>
#include <BRepAlgoAPI_Cut.hxx>
#include <BRepGProp.hxx>
#include <GProp_GProps.hxx>
#include <TopoDS_Shape.hxx>
#include <cmath>
#include <chrono>
#include <filesystem>
#include <iostream>
#include <stdexcept>

// Independent oracle deliberately permits empty difference solids; production
// Shape::checked rejects zero-volume outputs and cannot serve this comparison.
static TopoDS_Shape raw_step(const std::filesystem::path& path) {
  STEPControl_Reader reader;
  if (reader.ReadFile(path.string().c_str()) != IFSelect_RetDone || reader.TransferRoots() < 1)
    throw std::runtime_error("Oracle STEP read failed");
  return reader.OneShape();
}
static double difference_volume(const TopoDS_Shape& a, const TopoDS_Shape& b) {
  BRepAlgoAPI_Cut difference(a, b);
  difference.Build();
  if (!difference.IsDone()) throw std::runtime_error("Oracle difference failed");
  GProp_GProps measure;
  BRepGProp::VolumeProperties(difference.Shape(), measure);
  return std::abs(measure.Mass());
}
int main() {
  const auto directory = std::filesystem::temp_directory_path() /
    ("forma-topology-oracle-" + std::to_string(std::chrono::steady_clock::now().time_since_epoch().count()));
  std::filesystem::create_directory(directory);
  try {
    for (double width : {40., 80.}) for (double angle : {17., 61.}) {
      std::cerr << "Oracle width=" << width << " angle=" << angle << " seed\n";
      auto seed = forma::make_referenced_box(width, 20, 10, "pad");
      if (!seed->ok()) throw std::runtime_error(seed->error_code());
      auto moved = forma::translate(*seed, 27, -13, 9);
      moved = forma::topology_occurrence(*moved, "offset");
      auto rotated = forma::rotate(*moved, 3, 5, 7, 1, 2, 3, angle);
      rotated = forma::topology_occurrence(*rotated, "spin");
      auto mirrored = forma::mirror(*rotated, 4, -7, 2, 1, -2, 3);
      mirrored = forma::topology_occurrence(*mirrored, "mirror");
      std::cerr << "Reference fillet\n";
      auto actual = forma::fillet_referenced_edge(*mirrored, "pad", "box-edge:x:ymin:zmax",
        "offset/spin/mirror", 1.25);
      // Separate local selector before transformation, not the new resolver.
      auto local = forma::make_box(width, 20, 10);
      auto rounded = forma::fillet_edge(*local, "box-edge:x:ymin:zmax", 1.25);
      auto translated = forma::translate(*rounded, 27, -13, 9);
      auto expected_rotation = forma::rotate(*translated, 3, 5, 7, 1, 2, 3, angle);
      auto expected = forma::mirror(*expected_rotation, 4, -7, 2, 1, -2, 3);
      std::cerr << "Export and raw STEP comparison\n";
      if (!actual->ok() || !expected->ok()) throw std::runtime_error(actual->error_code());
      std::string error;
      const auto a = directory / "actual.step", b = directory / "expected.step";
      if (!forma::write_step(*actual, a.string(), error) || !forma::write_step(*expected, b.string(), error))
        throw std::runtime_error("Oracle STEP export failed");
      const auto raw_a = raw_step(a), raw_b = raw_step(b);
      const auto symmetric_difference = difference_volume(raw_a, raw_b) + difference_volume(raw_b, raw_a);
      if (symmetric_difference > 1e-5) throw std::runtime_error("Reference moved onto a different edge");
      const double analytic = width * 20 * 10 - width * 1.25 * 1.25 * (1 - std::acos(-1.) / 4);
      if (std::abs(actual->volume_mm3() - analytic) > 1e-5) throw std::runtime_error("Analytic fillet mismatch");
      auto wrong = forma::fillet_referenced_edge(*mirrored, "pad", "box-edge:x:ymin:zmax", "other/spin/mirror", 1.25);
      if (wrong->ok() || wrong->error_code() != "TOPOLOGY_REFERENCE_UNRESOLVED")
        throw std::runtime_error("Wrong branch accepted");
    }
    std::filesystem::remove_all(directory);
    return 0;
  } catch (const std::exception& error) {
    std::cerr << error.what() << '\n';
    std::filesystem::remove_all(directory);
    return 1;
  }
}
