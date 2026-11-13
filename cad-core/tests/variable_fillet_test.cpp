#include "forma_variable_fillet.hpp"
#include "test_support.hpp"
#include <chrono>
#include <cmath>
#include <filesystem>
#include <iostream>
#include <iomanip>
#include <limits>
#include <numeric>

int main() {
  return run_test([] {
    const auto box=forma::make_referenced_box(40.,20.,10.,"pad");
    const std::string role="box-edge:z:xmax:ymax";
    const auto endpoints=forma::variable_fillet_endpoints(*box,"pad",role,"");
    require(endpoints && endpoints->valid(),"Exact open edge endpoints");
    require(endpoints->start_mm().size()==3 && endpoints->end_mm().size()==3,"Endpoint vectors");
    require(std::abs(endpoints->start_mm()[2]-endpoints->end_mm()[2])==10.,"Endpoint span");
    for (const auto radii : {std::pair{1.,3.},std::pair{3.,1.},std::pair{2.,2.},std::pair{1.25,3.5}}) {
      const auto result=forma::variable_fillet(*box,"pad",role,"",radii.first,radii.second);
      require(result && result->ok(),result ? result->error_message().c_str() : "Null fillet");
      std::cout<<std::setprecision(15)<<"radii "<<radii.first<<","<<radii.second<<" V "<<result->volume_mm3()<<" A "<<result->surface_area_mm2()<<"\n";
      // A variable rolling-ball fillet has oblique end sections; integrating
      // circular XY sectors would be an incorrect oracle for unequal radii.
      const double factor=(1.-std::acos(-1.)/4.)*10.;
      if (radii.first==radii.second)
        require(std::abs(result->volume_mm3()-(8000.-factor*4.))<1e-5,"Constant-radius analytic volume");
      else
        require(result->volume_mm3()>8000.-factor*std::pow(std::max(radii.first,radii.second),2)
            && result->volume_mm3()<8000.-factor*std::pow(std::min(radii.first,radii.second),2),"Variable volume between endpoint-radius bounds");
      if (radii.first!=radii.second) {
        const double start_z=endpoints->start_mm()[2]<5. ? .05 : 9.95;
        const double end_z=10.-start_z;
        const auto at_start=forma::section_plane(*result,0.,0.,start_z,0.,0.,1.,.001);
        const auto at_end=forma::section_plane(*result,0.,0.,end_z,0.,0.,1.,.001);
        require(at_start && at_start->valid() && at_end && at_end->valid(),"Independent end sections");
        const auto perimeter=[](const auto& section) { const auto& lengths=section.edge_lengths_mm(); return std::accumulate(lengths.begin(),lengths.end(),0.); };
        require((perimeter(*at_start)-perimeter(*at_end))*(radii.second-radii.first)>0.,"Larger radius reaches the explicitly bound endpoint");
      }
      require(result->face_count()==7,"Seven faces after single-edge fillet");
      const auto path=std::filesystem::temp_directory_path()/
          ("forma-variable-fillet-"+std::to_string(std::chrono::steady_clock::now().time_since_epoch().count())+".step");
      std::string error;
      require(forma::write_step(*result,path.string(),error),"STEP export");
      const auto restored=forma::read_step(path.string());
      std::filesystem::remove(path);
      require(restored && restored->ok(),"STEP BREP validity");
      require(std::abs(restored->volume_mm3()-result->volume_mm3())<1e-3,"STEP volume");
      require(std::abs(restored->surface_area_mm2()-result->surface_area_mm2())<1e-3,"STEP area");
    }
    for (double bad : {0.,-1.,10001.,std::numeric_limits<double>::quiet_NaN()})
      require(!forma::variable_fillet(*box,"pad",role,"",bad,2.)->ok(),"Invalid radius rejected");
    require(!forma::variable_fillet(*box,"pad",role,"",50.,50.)->ok(),"Oversized radius rejected");
    require(!forma::variable_fillet(*box,"wrong",role,"",1.,3.)->ok(),"Wrong owner rejected");
    require(!forma::variable_fillet(*box,"pad",role,"wrong",1.,3.)->ok(),"Wrong path rejected");
    const auto cylinder=forma::make_referenced_cylinder(10.,10.,"cylinder");
    const auto closed=forma::variable_fillet(*cylinder,"cylinder","cylinder-edge:top","",1.,3.);
    require(!closed->ok() && closed->error_code()=="VARIABLE_FILLET_CLOSED_EDGE","Closed edge explicitly rejected");
    const auto moved=forma::translate(*box,27.,-13.,9.);
    const auto occurrence=forma::topology_occurrence(*moved,"move");
    const auto rounded=forma::variable_fillet(*occurrence,"pad",role,"move",1.,3.);
    require(rounded->ok(),"Occurrence resolves after translation");
    const auto moved_ends=forma::variable_fillet_endpoints(*occurrence,"pad",role,"move");
    for (std::size_t axis=0;axis<3;++axis) {
      const double shift=axis==0 ? 27. : axis==1 ? -13. : 9.;
      require(std::abs(moved_ends->start_mm()[axis]-endpoints->start_mm()[axis]-shift)<1e-8,"Start identity survives translation");
      require(std::abs(moved_ends->end_mm()[axis]-endpoints->end_mm()[axis]-shift)<1e-8,"End identity survives translation");
    }
    const auto rotated=forma::rotate(*box,0.,0.,0.,0.,0.,1.,90.);
    const auto turned=forma::topology_occurrence(*rotated,"turn");
    const auto reflected=forma::mirror(*turned,0.,0.,0.,1.,0.,0.);
    const auto placed=forma::topology_occurrence(*reflected,"reflect");
    const auto reflected_ends=forma::variable_fillet_endpoints(*placed,"pad",role,"turn/reflect");
    require(reflected_ends && reflected_ends->valid(),"Endpoints survive rotation and reflection");
    require(std::abs(reflected_ends->start_mm()[0]-endpoints->start_mm()[1])<1e-8,"Reflected first vertex x");
    require(std::abs(reflected_ends->start_mm()[1]-endpoints->start_mm()[0])<1e-8,"Rotated first vertex y");
    require(std::abs(reflected_ends->start_mm()[2]-endpoints->start_mm()[2])<1e-8,"First vertex identity is not world-sorted");
    const auto transformed=forma::variable_fillet(*placed,"pad",role,"turn/reflect",1.,3.);
    require(transformed->ok(),"Variable fillet after rotation and reflection");
    require(std::abs(transformed->volume_mm3()-rounded->volume_mm3())<1e-3,"Rigid transforms retain fillet volume");
    require(std::abs(box->volume_mm3()-8000.)<1e-9,"Source remains unchanged");
  });
}
