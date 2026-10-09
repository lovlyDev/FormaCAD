#include "forma_core.hpp"
#include <cmath>
#include <limits>
int main() {
  auto box=forma::make_box(40,20,10);
  auto section=forma::section_plane(*box,0,0,5,0,0,7,0.01);
  if (!section || !section->valid() || section->edge_lengths_mm().size()!=4) return 1;
  double total=0;for (double length:section->edge_lengths_mm()) total+=length;
  if (std::abs(total-120)>1e-6 || std::abs(box->volume_mm3()-8000)>1e-6) return 2;
  auto empty=forma::section_plane(*box,0,0,50,0,0,1,0.01);
  if (!empty || !empty->valid() || !empty->edge_lengths_mm().empty() || !empty->points_mm().empty()) return 3;
  if (forma::section_plane(*box,0,0,5,0,0,0,0.01)->valid()) return 4;
  if (forma::section_plane(*box,0,0,5,0,0,1,0)->valid()) return 5;
  if (forma::section_plane(*box,0,0,5,0,0,std::numeric_limits<double>::infinity(),0.01)->valid()) return 6;
  auto outer=forma::make_cylinder(10,20);auto inner=forma::make_cylinder(4,20);auto hollow=forma::boolean_cut(*outer,*inner);
  auto ring=forma::section_plane(*hollow,0,0,7,0,0,1,0.001);
  if (!ring || !ring->valid() || ring->edge_lengths_mm().size()!=2) return 7;
  total=0;for (double length:ring->edge_lengths_mm()) total+=length;
  if (std::abs(total-28*std::acos(-1.))>1e-6) return 8;
  return 0;
}
