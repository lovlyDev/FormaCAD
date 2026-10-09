// Independent analytic oracles for exact trimmed-entity pair measurements.
#include "forma_core.hpp"
#include "forma_pair_measurement.hpp"
#include <algorithm>
#include <array>
#include <cmath>
#include <iostream>
#include <limits>
#include <stdexcept>

namespace {
struct Ref { int kind; std::string role; std::string path; std::string owner = "pad"; };
void require(bool condition, const char* detail) {
  if (!condition) throw std::runtime_error(detail);
}
void near(double actual, double expected) {
  require(std::isfinite(actual) && std::abs(actual - expected) <= 1e-6 * std::max(1.0, std::abs(expected)),
    "Independent analytic pair measurement failed");
}
std::unique_ptr<forma::Shape> shape(std::unique_ptr<forma::Shape> value) {
  require(value && value->ok(), "Invalid authored fixture"); return value;
}
std::unique_ptr<forma::PairMeasurementResult> measured(const forma::Shape& body, int kind, Ref a, Ref b) {
  auto result = forma::measure_reference_pair(body, kind, a.kind, a.owner, a.role, a.path,
    b.kind, b.owner, b.role, b.path);
  require(result && result->valid() && result->query_kind() == kind, "Exact pair resolution failed");
  return result;
}
void distance(const forma::Shape& body, Ref a, Ref b, double expected) {
  const auto result = measured(body, 0, a, b);
  near(result->distance_mm(), expected);
  require(result->point_a_mm().size() == 3 && result->point_b_mm().size() == 3, "Missing minimum witness pair");
  double squared = 0;
  for (int axis = 0; axis < 3; ++axis)
    squared += std::pow(result->point_a_mm()[axis] - result->point_b_mm()[axis], 2);
  near(std::sqrt(squared), expected);
  near(measured(body, 0, b, a)->distance_mm(), expected); // Scalar symmetry, witnesses need not be canonical.
}
void angle(const forma::Shape& body, int kind, Ref a, Ref b, double expected) {
  near(measured(body, kind, a, b)->angle_deg(), expected);
  near(measured(body, kind, b, a)->angle_deg(), expected);
}
void rejected(const forma::Shape& body, int kind, Ref a, Ref b, const char* code) {
  auto result = forma::measure_reference_pair(body, kind, a.kind, a.owner, a.role, a.path,
    b.kind, b.owner, b.role, b.path);
  require(result && !result->valid() && result->error_code() == code, "Incorrect safe pair rejection");
}
void box_analytics(const forma::Shape& body, double width, double depth, double height, const std::string& path) {
  const Ref xmin{1,"box-face:xmin",path}, xmax{1,"box-face:xmax",path},
    ymin{1,"box-face:ymin",path}, ymax{1,"box-face:ymax",path},
    zmin{1,"box-face:zmin",path}, zmax{1,"box-face:zmax",path};
  const Ref edge_low{0,"box-edge:x:ymin:zmin",path}, edge_up{0,"box-edge:x:ymin:zmax",path},
    edge_far{0,"box-edge:x:ymax:zmax",path}, crossing{0,"box-edge:y:xmin:zmin",path};
  distance(body, xmin, xmax, width); distance(body, ymin, ymax, depth);
  distance(body, zmin, zmax, height); distance(body, zmax, zmax, 0);
  distance(body, xmin, ymin, 0); distance(body, edge_low, edge_low, 0);
  distance(body, edge_low, edge_up, height);
  distance(body, edge_low, edge_far, std::hypot(depth,height));
  distance(body, edge_low, zmax, height); distance(body, edge_low, crossing, 0);
  angle(body, 1, xmin, xmax, 180); angle(body, 1, zmin, zmax, 180);
  angle(body, 1, xmin, ymin, 90); angle(body, 1, zmax, zmax, 0);
  angle(body, 2, edge_low, edge_far, 0); angle(body, 2, edge_low, crossing, 90);
}
void run() {
  auto box = shape(forma::make_referenced_box(40,20,10,"pad"));
  box_analytics(*box,40,20,10,"");
  auto turned = shape(forma::rotate(*box,2,3,4,0,1,0,31));
  turned = shape(forma::topology_occurrence(*turned,"turn"));
  auto moved = shape(forma::translate(*turned,7,-3,11));
  moved = shape(forma::topology_occurrence(*moved,"move"));
  const double inv = 1/std::sqrt(3.0);
  auto reflected = shape(forma::mirror(*moved,2,3,4,inv,inv,inv));
  reflected = shape(forma::topology_occurrence(*reflected,"flip"));
  box_analytics(*reflected,40,20,10,"turn/move/flip");
  rejected(*reflected,0,{1,"box-face:xmin","turn/move/flip"},
    {1,"box-face:xmax","turn/move"},"TOPOLOGY_REFERENCE_UNRESOLVED");
  rejected(*reflected,0,{1,"box-face:xmin","turn/move/flip"},
    {1,"box-face:xmax","turn/move/flip","other_pad"},"TOPOLOGY_REFERENCE_UNRESOLVED");
  auto resized = shape(forma::make_referenced_box(52,30,17,"pad"));
  box_analytics(*resized,52,30,17,"");
  auto cylinder = shape(forma::make_referenced_cylinder(8,25,"pad"));
  const Ref top_edge{0,"cylinder-edge:top",""}, bottom_edge{0,"cylinder-edge:bottom",""},
    top_face{1,"cylinder-face:top",""}, bottom_face{1,"cylinder-face:bottom",""},
    side_face{1,"cylinder-face:side",""};
  distance(*cylinder,top_edge,bottom_edge,25); distance(*cylinder,top_face,bottom_face,25);
  distance(*cylinder,top_edge,bottom_face,25); distance(*cylinder,side_face,bottom_edge,0);
  angle(*cylinder,1,top_face,bottom_face,180); angle(*cylinder,1,top_face,top_face,0);
  auto cylinder_turned = shape(forma::rotate(*cylinder,2,3,4,0,1,0,31));
  cylinder_turned = shape(forma::topology_occurrence(*cylinder_turned,"turn"));
  auto cylinder_moved = shape(forma::translate(*cylinder_turned,7,-3,11));
  cylinder_moved = shape(forma::topology_occurrence(*cylinder_moved,"move"));
  auto cylinder_mirrored = shape(forma::mirror(*cylinder_moved,2,3,4,inv,inv,inv));
  cylinder_mirrored = shape(forma::topology_occurrence(*cylinder_mirrored,"flip"));
  const Ref transformed_top{0,"cylinder-edge:top","turn/move/flip"},
    transformed_bottom{0,"cylinder-edge:bottom","turn/move/flip"},
    transformed_top_face{1,"cylinder-face:top","turn/move/flip"},
    transformed_bottom_face{1,"cylinder-face:bottom","turn/move/flip"},
    transformed_side{1,"cylinder-face:side","turn/move/flip"};
  distance(*cylinder_mirrored,transformed_top,transformed_bottom,25);
  distance(*cylinder_mirrored,transformed_top_face,transformed_bottom_face,25);
  distance(*cylinder_mirrored,transformed_top,transformed_bottom_face,25);
  distance(*cylinder_mirrored,transformed_side,transformed_bottom,0);
  angle(*cylinder_mirrored,1,transformed_top_face,transformed_bottom_face,180);
  rejected(*cylinder_mirrored,2,transformed_top,transformed_bottom,"MEASUREMENT_UNSUPPORTED");
  rejected(*cylinder_mirrored,1,transformed_top_face,transformed_side,"MEASUREMENT_UNSUPPORTED");
  rejected(*cylinder_mirrored,0,transformed_top,bottom_edge,"TOPOLOGY_REFERENCE_UNRESOLVED");
  auto cylinder_resized = shape(forma::make_referenced_cylinder(12,37,"pad"));
  distance(*cylinder_resized,top_edge,bottom_edge,37);
  distance(*cylinder_resized,top_face,bottom_face,37);
  rejected(*cylinder,2,top_edge,bottom_edge,"MEASUREMENT_UNSUPPORTED");
  rejected(*cylinder,1,top_face,side_face,"MEASUREMENT_UNSUPPORTED");
  rejected(*cylinder,1,top_edge,bottom_face,"INVALID_MEASUREMENT_QUERY");
  rejected(*cylinder,0,top_face,{1,"cylinder-face:top","","foreign"},"TOPOLOGY_REFERENCE_UNRESOLVED");
  rejected(*cylinder,0,top_edge,{0,"cylinder-edge:bottom","wrong_branch"},"TOPOLOGY_REFERENCE_UNRESOLVED");
  rejected(*cylinder,0,top_edge,{0,"cylinder-edge:seam",""},"INVALID_TOPOLOGY_REFERENCE");
  rejected(*cylinder,3,top_edge,bottom_edge,"INVALID_MEASUREMENT_QUERY");
  rejected(*box,0,{2,"box-face:xmin",""},{1,"box-face:xmax",""},"INVALID_MEASUREMENT_QUERY");
  auto plain = shape(forma::make_cylinder(8,25));
  rejected(*plain,0,top_edge,bottom_edge,"TOPOLOGY_REFERENCE_UNSUPPORTED");
  auto modified = shape(forma::boolean_union(*box,*cylinder));
  rejected(*modified,0,{1,"box-face:xmin",""},{1,"box-face:xmax",""},"TOPOLOGY_REFERENCE_UNSUPPORTED");
  near(box->volume_mm3(),8000); near(cylinder->volume_mm3(),std::acos(-1.0)*64*25);
  box_analytics(*box,40,20,10,""); // Queries are read-only and preserve provenance.
}
}
int main() {
  try { run(); std::cout<<"Exact pair constructor analytics passed\n";return 0; }
  catch(const std::exception& error){std::cerr<<error.what()<<'\n';return 1;}
}
