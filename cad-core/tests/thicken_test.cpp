#include "forma_thicken.hpp"
#include "test_support.hpp"
#include <chrono>
#include <cmath>
#include <filesystem>
#include <limits>
#include <iostream>
#include <BRepBndLib.hxx>
#include <BRepCheck_Analyzer.hxx>
#include <Bnd_Box.hxx>
#include <STEPControl_Reader.hxx>
namespace {
void round_trip(const forma::Shape& source, double volume, double area, double zmin, double zmax) {
  const auto folder = std::filesystem::temp_directory_path() / ("forma-thicken-" + std::to_string(std::chrono::steady_clock::now().time_since_epoch().count()));
  require(std::filesystem::create_directory(folder), "own temporary directory");
  const auto path = folder / "round-trip.step";
  std::string error;
  require(forma::write_step(source, path.string(), error), "thicken STEP export");
  STEPControl_Reader reader;
  require(reader.ReadFile(path.string().c_str()) == IFSelect_RetDone && reader.TransferRoots() > 0, "read STEP roots");
  require(BRepCheck_Analyzer(reader.OneShape()).IsValid(), "STEP BREP validity");
  Bnd_Box bounds;
  BRepBndLib::Add(reader.OneShape(), bounds);
  double x0, y0, z0, x1, y1, z1;
  bounds.Get(x0, y0, z0, x1, y1, z1);
  require(std::abs(z0-zmin) < 1e-6 && std::abs(z1-zmax) < 1e-6, "offset follows oriented face normal");
  const auto restored = forma::read_step(path.string());
  std::filesystem::remove(path);
  std::filesystem::remove(folder);
  require(restored->ok(), "thicken STEP import");
  require(std::abs(restored->volume_mm3() - volume) < 1e-4, "STEP volume");
  require(std::abs(restored->surface_area_mm2() - area) < 1e-4, "STEP area");
}
}
int main() {
  return run_test([] {
    const auto source = forma::make_referenced_box(40., 20., 10., "pad");
    for (const double offset : {2., -2.}) {
      const auto plate = forma::thicken_face(*source, "pad", "box-face:zmax", "", offset);
      require(plate->ok(), plate->error_message().c_str());
      require(std::abs(plate->volume_mm3() - 1600.) < 1e-6, "plate volume");
      require(std::abs(plate->surface_area_mm2() - 1840.) < 1e-6, "plate area");
      require(std::abs(plate->extent_z_mm() - 2.) < 1e-6, "plate height");
      require(plate->face_count() == 6, "plate faces");
      round_trip(*plate, 1600., 1840., offset > 0 ? 10. : 8., offset > 0 ? 12. : 10.);
    }
    const auto bottom = forma::thicken_face(*source, "pad", "box-face:zmin", "", 2.);
    require(bottom->ok() && std::abs(bottom->volume_mm3() - 1600.) < 1e-6, "reversed face");
    round_trip(*bottom, 1600., 1840., -2., 0.);
    require(std::abs(source->volume_mm3() - 8000.) < 1e-6, "source unchanged");
    const auto turned = forma::rotate(*source, 0., 0., 0., 1., 0., 0., 90.);
    const auto occurrence = forma::topology_occurrence(*turned, "turn");
    const auto rotated_plate = forma::thicken_face(*occurrence, "pad", "box-face:zmax", "turn", 2.);
    require(rotated_plate->ok(), "rotated face reference");
    require(std::abs(rotated_plate->volume_mm3()-1600.) < 1e-6, "rotated plate volume");
    require(std::abs(rotated_plate->extent_y_mm()-2.) < 1e-6, "rotated face normal");
    round_trip(*rotated_plate, 1600., 1840., -10., 10.);

    require(!forma::thicken_face(*source, "wrong", "box-face:zmax", "", 2.)->ok(), "stale owner rejected");
    require(!forma::thicken_face(*source, "pad", "box-face:zmax", "missing", 2.)->ok(), "stale occurrence rejected");
    require(!forma::thicken_face(*source, "pad", "box-face:missing", "", 2.)->ok(), "missing face rejected");
    for (const double offset : {0., 10001., std::numeric_limits<double>::quiet_NaN()}) {
      const auto invalid = forma::thicken_face(*source, "pad", "box-face:zmax", "", offset);
      require(!invalid->ok() && invalid->error_code() == "INVALID_THICKNESS", "invalid thickness rejected");
    }
    const double pi = std::acos(-1.);
    const auto cylinder = forma::make_referenced_cylinder(10., 20., "cylinder");
    for (const double offset : {2., -2.}) {
      const auto ring = forma::thicken_face(*cylinder, "cylinder", "cylinder-face:side", "", offset);
      require(ring->ok(), ring->error_message().c_str());
      std::cout << "Curved thicken " << offset << ": " << ring->volume_mm3() << " mm3, " << ring->surface_area_mm2() << " mm2\n";
      const double expected = offset > 0 ? 880. * pi : 720. * pi;
      require(std::abs(ring->volume_mm3() - expected) < 1e-4, "curved face volume");
      const double area = offset > 0 ? 968. * pi : 792. * pi;
      require(std::abs(ring->surface_area_mm2() - area) < 1e-4, "curved face area");
      require(ring->face_count() == 4, "curved face topology");
      round_trip(*ring, expected, area, 0., 20.);
    }
    const auto impossible = forma::thicken_face(*cylinder, "cylinder", "cylinder-face:side", "", -12.);
    require(!impossible->ok(), "collapsed cylinder offset rejected");
    require(std::abs(cylinder->volume_mm3() - 2000. * pi) < 1e-4, "curved source unchanged");
  });
}
