#include "forma_core.hpp"
#include "forma_split.hpp"
#include "test_support.hpp"
#include <cmath>
#include <chrono>
#include <filesystem>
#include <limits>
int main() {
  return run_test([] {
    const auto body = forma::make_box(40., 20., 10.);
    const auto positive = forma::split_by_plane(*body, 5., 0., 0., 1., 0., 0., true);
    const auto negative = forma::split_by_plane(*body, 5., 0., 0., 1., 0., 0., false);
    require(positive && negative && positive->ok() && negative->ok(), "plane split failed");
    require(std::abs(positive->volume_mm3() - 3000.) < 1e-6, "positive half volume");
    require(std::abs(negative->volume_mm3() - 5000.) < 1e-6, "negative half volume");
    require(positive->face_count() == 6 && negative->face_count() == 6, "split face topology");
    require(std::abs(positive->extent_x_mm() - 15.) < 1e-6, "positive half bounds");
    require(std::abs(negative->extent_x_mm() - 25.) < 1e-6, "negative half bounds");
    require(std::abs(body->volume_mm3() - 8000.) < 1e-6, "split changed source");
    const double pi = std::acos(-1.);
    const auto cylinder = forma::make_cylinder(10., 20.);
    const auto cylinder_top = forma::split_by_plane(*cylinder, 0., 0., 5., 0., 0., 1., true);
    const auto cylinder_bottom = forma::split_by_plane(*cylinder, 0., 0., 5., 0., 0., 1., false);
    require(cylinder_top->ok() && cylinder_bottom->ok(), "cylinder split");
    require(std::abs(cylinder_top->volume_mm3() - 1500. * pi) < 1e-4, "cylinder upper volume");
    require(std::abs(cylinder_bottom->volume_mm3() - 500. * pi) < 1e-4, "cylinder lower volume");
    const auto sphere = forma::make_sphere(10.);
    const auto hemisphere = forma::split_by_plane(*sphere, 0., 0., 0., 1., 0., 0., true);
    require(hemisphere->ok(), "sphere split");
    require(std::abs(hemisphere->volume_mm3() - 2000. * pi / 3.) < 1e-4, "hemisphere volume");
    require(std::abs(hemisphere->surface_area_mm2() - 300. * pi) < 1e-4, "hemisphere surface area");
    const auto missed = forma::split_by_plane(*body, 100., 0., 0., 1., 0., 0., true);
    const auto tangent = forma::split_by_plane(*body, 20., 0., 0., 1., 0., 0., false);
    const auto invalid = forma::split_by_plane(*body, 0., 0., 0., 0., 0., 0., true);
    require(!missed->ok() && missed->error_code() == "SPLIT_NO_INTERSECTION", "missed plane must fail");
    require(!tangent->ok() && tangent->error_code() == "SPLIT_NO_INTERSECTION", "tangent plane must fail");
    require(!invalid->ok() && invalid->error_code() == "INVALID_SPLIT_PLANE", "zero normal must fail");
    const auto diagonal = forma::split_by_plane(*body, 0., 0., 0., 1., 1., 0., true);
    require(diagonal->ok() && std::abs(diagonal->volume_mm3() - 4000.) < 1e-6, "oblique half volume");
    const auto nonfinite = forma::split_by_plane(*body, std::numeric_limits<double>::quiet_NaN(), 0., 0., 1., 0., 0., true);
    require(!nonfinite->ok(), "nonfinite plane must fail");
    const auto folder = std::filesystem::temp_directory_path() / ("forma-split-" + std::to_string(std::chrono::steady_clock::now().time_since_epoch().count()));
    require(std::filesystem::create_directory(folder), "own temporary directory");
    const auto path = folder / "round-trip.step";
    std::string error;
    require(forma::write_step(*positive, path.string(), error), "split STEP export");
    const auto restored = forma::read_step(path.string());
    std::filesystem::remove(path);
    std::filesystem::remove(folder);
    require(restored->ok() && std::abs(restored->volume_mm3() - 3000.) < 1e-6, "split STEP round trip");
  });
}
