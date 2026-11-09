#include "forma_core.hpp"
#include "test_support.hpp"
#include <cmath>

int main() {
  return run_test([] {
    const auto box = forma::make_box(40., 20., 10.);
    require(box->ok(), "box construction");
    for (std::uint32_t face = 1; face <= 6; ++face) {
      const auto result = forma::smart_select(*box, 0, face);
      require(result && result->size() == 2, "opposite planar faces share an exact area");
    }
    for (std::uint32_t edge = 1; edge <= 12; ++edge) {
      const auto similar = forma::smart_select(*box, 1, edge);
      const auto tangent = forma::smart_select(*box, 2, edge);
      require(similar && similar->size() == 4, "four edges have the same exact length");
      require(tangent && *tangent == std::vector<std::uint32_t>{edge}, "chain stops at a corner");
    }
    require(!forma::smart_select(*box, 0, 7), "invalid face accepted");
    require(!forma::smart_select(*box, 2, 0), "zero edge accepted");
    const auto cylinder = forma::make_cylinder(10., 20.);
    std::size_t matching_rims = 0;
    for (std::uint32_t edge = 1; edge <= static_cast<std::uint32_t>(cylinder->edge_count()); ++edge) {
      const auto similar = forma::smart_select(*cylinder, 1, edge);
      if (similar && similar->size() == 2) ++matching_rims;
    }
    require(matching_rims == 2, "two circular edges share an exact radius and length");
    const double coords[]{0,0,10,0,20,0,20,10,0,10};
    const std::size_t offsets[]{0,10};
    const auto split = forma::make_profile_prism(coords, 10, offsets, 2, 0, 0, 0, 0, 5);
    require(split->ok() && std::abs(split->volume_mm3() - 1000.) < 1e-8, "split prism construction");
    std::size_t joined = 0;
    for (std::uint32_t edge = 1; edge <= static_cast<std::uint32_t>(split->edge_count()); ++edge) {
      const auto result = forma::smart_select(*split, 2, edge);
      if (result && result->size() == 2) ++joined;
    }
    require(joined == 4, "collinear edge pairs link despite a perpendicular branch");
  });
}
