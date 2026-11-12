#include "forma_mirror_features.hpp"
#include "test_support.hpp"
#include <chrono>
#include <cmath>
#include <filesystem>
#include <limits>
namespace {
void round_trip(const forma::Shape& shape, double volume, double area) {
  const auto folder = std::filesystem::temp_directory_path() / ("forma-mirror-features-" + std::to_string(std::chrono::steady_clock::now().time_since_epoch().count()));
  require(std::filesystem::create_directory(folder), "own temporary folder");
  const auto path = folder / "model.step";
  std::string error;
  require(forma::write_step(shape, path.string(), error), "feature mirror STEP export");
  const auto restored = forma::read_step(path.string());
  std::filesystem::remove(path); std::filesystem::remove(folder);
  require(restored->ok(), "feature mirror STEP BREP");
  require(std::abs(restored->volume_mm3()-volume)<1e-4, "STEP volume");
  require(std::abs(restored->surface_area_mm2()-area)<1e-4, "STEP area");
}
}
int main() {
  return run_test([] {
    const double pi = std::acos(-1.);
    const auto base = forma::make_box(40.,20.,10.);
    const auto cylinder = forma::make_cylinder(2.,4.);
    const auto boss = forma::translate(*cylinder,10.,0.,10.);
    const auto source = forma::boolean_union(*base,*boss);
    require(source->ok(), "boss source");
    const auto mirrored = forma::mirror_features(*source,*base,true,0.,0.,0.,1.,0.,0.);
    require(mirrored->ok(), mirrored->error_message().c_str());
    require(std::abs(mirrored->volume_mm3()-(8000.+32.*pi))<1e-4,"mirrored boss volume");
    require(std::abs(mirrored->surface_area_mm2()-(2800.+32.*pi))<1e-4,"mirrored boss area");
    require(mirrored->face_count()==10 && mirrored->edge_count()==18,"boss topology");
    round_trip(*mirrored,8000.+32.*pi,2800.+32.*pi);
    require(std::abs(source->volume_mm3()-(8000.+16.*pi))<1e-4,"source remains unchanged");
    const auto cutter = forma::make_cylinder(2.,20.);
    const auto placed_cutter = forma::translate(*cutter,10.,0.,-5.);
    const auto cut = forma::boolean_cut(*base,*placed_cutter);
    const auto holes = forma::mirror_features(*cut,*base,true,0.,0.,0.,1.,0.,0.);
    require(holes->ok(), holes->error_message().c_str());
    require(std::abs(holes->volume_mm3()-(8000.-80.*pi))<1e-4,"mirrored hole volume");
    require(std::abs(holes->surface_area_mm2()-(2800.+64.*pi))<1e-4,"mirrored hole area");
    require(holes->face_count()==8,"hole topology");
    round_trip(*holes,8000.-80.*pi,2800.+64.*pi);
    const auto small_cutter = forma::make_cylinder(1.,20.);
    const auto small_placed = forma::translate(*small_cutter,5.,0.,-5.);
    const auto combined = forma::boolean_cut(*source,*small_placed);
    const auto both = forma::mirror_features(*combined,*base,true,0.,0.,0.,1.,0.,0.);
    require(both->ok(), both->error_message().c_str());
    require(std::abs(both->volume_mm3()-(8000.+12.*pi))<1e-4,"additive and subtractive range");
    round_trip(*both,8000.+12.*pi,2800.+68.*pi);
    const auto sphere = forma::make_sphere(10.);
    const auto generator = forma::mirror_features(*sphere,*sphere,false,5.,0.,0.,1.,0.,0.);
    require(generator->ok(), generator->error_message().c_str());
    round_trip(*generator,2250.*pi,600.*pi);
    const auto no_effect = forma::mirror_features(*source,*base,true,10.,0.,0.,1.,0.,0.);
    require(!no_effect->ok() && no_effect->error_code()=="MIRROR_FEATURES_NO_EFFECT","reject duplicate effect");
    const auto detached = forma::mirror_features(*source,*base,true,100.,0.,0.,1.,0.,0.);
    require(!detached->ok() && detached->error_code()=="MIRROR_FEATURES_DISCONNECTED","reject disconnected result");
    require(!forma::mirror_features(*source,*base,true,0.,0.,0.,0.,0.,0.)->ok(),"reject zero normal");
    require(!forma::mirror_features(*source,*base,true,std::numeric_limits<double>::quiet_NaN(),0.,0.,1.,0.,0.)->ok(),"reject NaN plane");
    require(std::abs(base->volume_mm3()-8000.)<1e-6,"base unchanged");
  });
}
