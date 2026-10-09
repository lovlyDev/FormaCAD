#include "forma_core.hpp"
#include "test_support.hpp"

int main() {
  return run_test([] {
    auto box = forma::make_box(60.0, 40.0, 10.0);
    require(box && box->ok(), "box construction failed");
    auto chamfered = forma::chamfer_all_edges(*box, 2.0);
    require(chamfered && chamfered->ok(), "all-edge chamfer failed");
    require(chamfered->volume_mm3() < box->volume_mm3(), "chamfer did not remove material");
    require(chamfered->volume_mm3() > 0.0, "chamfer produced an empty solid");
    auto invalid = forma::chamfer_all_edges(*box, 0.0);
    require(invalid && !invalid->ok() && invalid->error_code() == "INVALID_DIMENSION",
            "zero distance was accepted");
    auto oversized = forma::chamfer_all_edges(*box, 100.0);
    require(oversized && !oversized->ok(), "oversized chamfer was accepted");
  });
}
