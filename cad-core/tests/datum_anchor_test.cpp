#include "forma_core.hpp"
#include "forma_datum_anchor.hpp"
#include "test_support.hpp"

#include <array>
#include <cmath>

namespace {

void near(double actual, double expected, const char* detail) {
  require(std::abs(actual - expected) < 1e-7, detail);
}

} // namespace

int main() {
  return run_test([] {
    const auto box = forma::make_referenced_box(20, 10, 4, "pad");
    std::array<double, 6> frame{};
    require(forma::datum_anchor(*box, 0, "pad", "box-face:zmax", "", frame.data(), frame.size()),
      "Resolve planar face by semantic role");
    near(frame[0], 0., "Face centroid X");
    near(frame[1], 0., "Face centroid Y");
    near(frame[2], 4., "Face centroid Z");
    near(frame[5], 1., "Outward normal");
    require(forma::datum_anchor(*box, 1, "pad", "box-edge:x:ymin:zmax", "", frame.data(), frame.size()),
      "Resolve straight edge by semantic role");
    near(frame[0], 0., "Edge midpoint X");
    near(frame[1], -5., "Edge midpoint Y");
    near(frame[2], 4., "Edge midpoint Z");
    near(std::abs(frame[3]), 1., "Edge tangent");
    const auto moved = forma::topology_occurrence(*forma::translate(*box, 7., 1., 3.), "move");
    require(forma::datum_anchor(*moved, 0, "pad", "box-face:zmax", "move", frame.data(), frame.size()),
      "Resolve transformed face");
    near(frame[0], 7., "Translated face centroid X");
    near(frame[2], 7., "Translated face centroid Z");
    require(!forma::datum_anchor(*moved, 0, "pad", "box-face:zmax", "", frame.data(), frame.size()),
      "Reject an outdated occurrence");
    const auto cylinder = forma::make_referenced_cylinder(5, 12, "cylinder");
    require(!forma::datum_anchor(*cylinder, 0, "cylinder", "cylinder-face:side", "", frame.data(), frame.size()),
      "Reject curved face");
    require(!forma::datum_anchor(*cylinder, 1, "cylinder", "cylinder-edge:top", "", frame.data(), frame.size()),
      "Reject circular edge");
  });
}
