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
  });
}
