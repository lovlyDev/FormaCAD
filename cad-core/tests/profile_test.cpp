#include "forma_core.hpp"
#include <cmath>
#include <limits>
int main() {
  const double coords[]={0,0,20,0,20,10,0,10,5,2,5,6,9,6,9,2};
  const std::size_t offsets[]={0,8,16};
  for (int plane=0;plane<3;++plane) for (double distance : {-5.0,5.0}) {
    auto solid=forma::make_profile_prism(coords,16,offsets,3,plane,3,4,5,distance);
    if (!solid || !solid->ok() || std::abs(solid->volume_mm3()-920)>1e-6 || solid->face_count()!=10) return 1;
  }
  const std::size_t broken[]={0,10,8};
  if (forma::make_profile_prism(coords,16,broken,3,0,0,0,0,5)->ok()) return 2;
  if (forma::make_profile_prism(nullptr,16,offsets,3,0,0,0,0,5)->ok()) return 3;
  if (forma::make_profile_prism(coords,16,offsets,3,0,0,0,0,std::numeric_limits<double>::quiet_NaN())->ok()) return 4;
  const double ring[]={2,0,6,0,6,10,2,10};
  const std::size_t ring_offsets[]={0,8};
  auto revolved=forma::make_profile_revolve(ring,8,ring_offsets,2,1,3,4,5,3,4,5,0,0,1,360);
  if (!revolved || !revolved->ok() || std::abs(revolved->volume_mm3()-320*std::acos(-1.0))>1e-6) return 5;
  if (forma::make_profile_revolve(nullptr,8,ring_offsets,2,1,3,4,5,3,4,5,0,0,1,360)->ok()) return 6;
  if (forma::make_profile_revolve(ring,8,ring_offsets,2,1,3,4,5,3,4,5,0,0,0,360)->ok()) return 7;
  if (forma::make_profile_revolve(ring,8,ring_offsets,2,1,3,4,5,3,5,5,0,0,1,360)->ok()) return 8;
  if (forma::make_profile_revolve(ring,8,ring_offsets,2,1,3,4,5,7,4,5,0,0,1,360)->ok()) return 9;
  if (forma::make_profile_revolve(ring,8,ring_offsets,2,1,3,4,5,3,4,5,0,0,1,0)->ok()) return 10;
  return 0;
}
