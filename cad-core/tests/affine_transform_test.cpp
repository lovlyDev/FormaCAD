#include "forma_core.hpp"
#include "test_support.hpp"
#include <BRepCheck_Analyzer.hxx>
#include <STEPControl_Reader.hxx>
#include <IFSelect_ReturnStatus.hxx>
#include <TopoDS_Shape.hxx>
#include <array>
#include <chrono>
#include <cmath>
#include <filesystem>
#include <limits>

static void near(double actual, double expected, const char* message) {
  require(std::abs(actual - expected) < 1e-5, message);
}
int main() {
  return run_test([] {
    const std::array<double, 12> identity{1,0,0,0, 0,1,0,0, 0,0,1,0};
    auto original = forma::make_referenced_box(20, 10, 5, "pad");
    require(original->ok(), "Referenced source");
    auto move = identity; move[3] = 10;
    auto moved = forma::transform_affine(*original, move.data(), move.size());
    require(moved->ok(), "10 mm translation");
    near(moved->volume_mm3(), 1000, "Translation volume invariant");
    near(original->volume_mm3(), 1000, "Source is immutable");
    auto original_plane = forma::measure_reference(*original, 4, "pad", "box-face:xmin", "");
    auto moved_plane = forma::measure_reference(*moved, 4, "pad", "box-face:xmin", "");
    require(original_plane->valid() && moved_plane->valid(), "Planar face remains analytic");
    near(moved_plane->origin_mm()[0] - original_plane->origin_mm()[0], 10, "Independent exact X displacement");
    auto cylinder = forma::make_referenced_cylinder(5, 10, "cylinder");
    auto uniform = identity; uniform[0] = 2; uniform[5] = 2; uniform[10] = 2;
    auto doubled = forma::transform_affine(*cylinder, uniform.data(), uniform.size());
    require(doubled->ok(), "Uniform cylinder scale");
    auto radius = forma::measure_reference(*doubled, 2, "cylinder", "cylinder-edge:top", "");
    require(radius->valid(), "Uniform scale preserves analytic circle");
    near(radius->radius_mm(), 10, "Exact scaled radius");

    // Independent oriented-scale oracle: scale X by 2, Y by 3, then rotate 90 degrees.
    const std::array<double, 12> matrix{0,-3,0,10, 2,0,0,-5, 0,0,4,7};
    auto scaled = forma::transform_affine(*original, matrix.data(), matrix.size());
    require(scaled->ok(), "Oriented nonuniform scale");
    near(scaled->volume_mm3(), 24000, "Determinant times volume");
    near(scaled->extent_x_mm(), 30, "Rotated X bounds");
    near(scaled->extent_y_mm(), 40, "Rotated Y bounds");
    near(scaled->extent_z_mm(), 20, "Scaled Z bounds");
    auto edge = forma::measure_reference(*scaled, 1, "pad", "box-edge:x:ymin:zmin", "");
    require(edge->valid(), "Semantic edge survives general transform");
    near(edge->length_mm(), 40, "Exact referenced edge length");

    const auto directory = std::filesystem::temp_directory_path() /
      ("forma-affine-" + std::to_string(std::chrono::steady_clock::now().time_since_epoch().count()));
    std::filesystem::create_directories(directory);
    const auto path = directory / "scaled.step";
    std::string error;
    require(forma::write_step(*scaled, path.string(), error), "Affine STEP export");
    STEPControl_Reader reader;
    require(reader.ReadFile(path.string().c_str()) == IFSelect_RetDone && reader.TransferRoots() > 0, "Independent STEP parse");
    require(BRepCheck_Analyzer(reader.OneShape()).IsValid(), "Round-trip BREP validity");
    auto roundtrip = forma::read_step(path.string());
    require(roundtrip->ok(), "STEP solid reload");
    near(roundtrip->volume_mm3(), 24000, "STEP volume invariant");
    // Only the unique test directory created above is removed.
    std::filesystem::remove(path);
    std::filesystem::remove(directory);

    for (double value : {0.0, -1.0, std::numeric_limits<double>::quiet_NaN(), std::numeric_limits<double>::infinity(), 1001.0}) {
      auto invalid = identity; invalid[0] = value;
      require(!forma::transform_affine(*original, invalid.data(), invalid.size())->ok(), "Reject invalid scale");
    }
    require(!forma::transform_affine(*original, nullptr, 12)->ok(), "Reject null transform");
    require(!forma::transform_affine(*original, identity.data(), 11)->ok(), "Reject incomplete transform");
    auto invalid = identity; invalid[3] = 10001;
    require(!forma::transform_affine(*original, invalid.data(), invalid.size())->ok(), "Reject excessive translation");
  });
}
