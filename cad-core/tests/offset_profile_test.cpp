#include "forma_offset_profile.hpp"
#include "test_support.hpp"
#include <chrono>
#include <cmath>
#include <filesystem>
#include <limits>
namespace {
void close(double actual,double expected){require(std::abs(actual-expected)<1e-4,"profile measurement");}
void exchange(const forma::Shape& shape){
 const auto directory=std::filesystem::temp_directory_path()/("forma-offset-"+std::to_string(std::chrono::steady_clock::now().time_since_epoch().count()));
 require(std::filesystem::create_directory(directory),"own directory");const auto path=directory/"profile.step";std::string error;
 require(forma::write_step(shape,path.string(),error),"STEP export");const auto restored=forma::read_step(path.string());
 require(restored->ok(),"STEP BREP");close(restored->volume_mm3(),shape.volume_mm3());close(restored->surface_area_mm2(),shape.surface_area_mm2());
 std::filesystem::remove(path);std::filesystem::remove(directory);
}
}
int main(){return run_test([]{
 const double coordinates[]={-20,-10,20,-10,20,10,-20,10,-5,-3,-5,3,5,3,5,-3};
 const std::size_t outer[]={0,8},holes[]={0,8,16};
 const auto base=forma::make_planar_profile(coordinates,8,outer,2,0,0,0,0);require(base->valid(),"rectangle profile");close(base->area_mm2(),800);close(base->perimeter_mm(),120);
 for(const int plane:{0,1,2})for(const double offset:{2.,-2.}){
  const auto source=forma::make_planar_profile(coordinates,8,outer,2,plane,100,200,30);
  const auto changed=forma::offset_profile(*source,offset,false);require(changed->valid(),changed->error_message().c_str());
  const auto a=(40+2*offset)*(20+2*offset);close(changed->area_mm2(),a);
  for(const double distance:{10.,-10.}){const auto solid=forma::extrude_planar_profile(*changed,distance);require(solid->ok(),solid->error_message().c_str());close(solid->volume_mm3(),a*10);exchange(*solid);}
  close(source->area_mm2(),800);
 }
 const auto rounded=forma::offset_profile(*base,2,true);require(rounded->valid(),"rounded offset");close(rounded->area_mm2(),1040+4*std::acos(-1.));
 exchange(*forma::extrude_planar_profile(*rounded,10));
 const auto circle=forma::make_circle_profile(10);for(const double d:{2.,-2.}){
  const auto changed=forma::offset_profile(*circle,d,true);require(changed->valid(),"circle offset");close(changed->area_mm2(),std::acos(-1.)*(10+d)*(10+d));exchange(*forma::extrude_planar_profile(*changed,10));
 }
 const auto perforated=forma::make_planar_profile(coordinates,16,holes,3,0,0,0,0);require(perforated->valid(),"perforated source");
 for(const double d:{2.,-2.}){const auto changed=forma::offset_profile(*perforated,d,false);require(changed->valid(),changed->error_message().c_str());require(changed->contour_count()==2,"retain hole");close(changed->area_mm2(),d>0?1044:436);exchange(*forma::extrude_planar_profile(*changed,10));}
 for(const double d:{4.,-5.}){const auto changed=forma::offset_profile(*perforated,d,false);require(!changed->valid()&&changed->error_code()=="PROFILE_OFFSET_TOPOLOGY_CHANGED","reject lost or external hole");}
 require(!forma::offset_profile(*base,-11,false)->valid(),"reject collapse");require(!forma::offset_profile(*circle,-12,true)->valid(),"reject circle collapse");
 for(const double d:{0.,std::numeric_limits<double>::quiet_NaN(),10001.})require(!forma::offset_profile(*base,d,false)->valid(),"invalid offset");
 close(perforated->area_mm2(),740);close(base->area_mm2(),800);
 // Offset profiles are also exact revolution inputs, not only extrusion recipes.
 const auto profile=forma::make_planar_profile(coordinates,8,outer,2,0,30,0,0);
 const auto changed=forma::offset_profile(*profile,2,false);
 const auto revolved=forma::revolve_planar_profile(*changed,0,0,0,0,1,0,360);
 require(revolved->ok(),revolved->error_message().c_str());close(revolved->volume_mm3(),1056*60*std::acos(-1.));exchange(*revolved);
});}
