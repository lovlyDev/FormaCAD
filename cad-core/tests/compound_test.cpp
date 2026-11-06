#include "forma_core.hpp"
#include "test_support.hpp"

#include <cmath>

int main() {
  return run_test([] {
    auto housing = forma::make_box(10.0, 20.0, 5.0);
    auto lid = forma::make_box(10.0, 20.0, 2.0);
    require(housing && lid && housing->ok() && lid->ok(), "body construction failed");
    auto offset_lid = forma::translate(*lid, 25.0, 0.0, 0.0);
    require(offset_lid && offset_lid->ok(), "body transform failed");
    auto assembly = forma::make_compound(*housing, *offset_lid);
    require(assembly && assembly->ok(), "compound construction failed");
    require(std::abs(assembly->volume_mm3() - 1400.0) < 0.001,
            "compound volume mismatch");
    require(assembly->face_count() == 12, "compound faces were lost");
    require(assembly->solid_count() == 2, "compound solid count mismatch");
    const auto first = forma::extract_solid(*assembly, 0, 2);
    const auto second = forma::extract_solid(*assembly, 1, 2);
    require(first->ok() && second->ok(), "solid extraction failed");
    require(std::abs(first->volume_mm3() - 1000.) < 1e-6 &&
            std::abs(second->volume_mm3() - 400.) < 1e-6, "solid extraction changed geometry");
    require(!forma::extract_solid(*assembly, 0, 3)->ok(), "stale solid count accepted");
  });
}
