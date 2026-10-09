#include "forma_core.hpp"
#include "test_support.hpp"

int main() {
  return run_test([] {
    auto box = forma::make_box(60.0, 40.0, 10.0);
    require(box && box->ok(), "box construction failed");
    auto rounded = forma::fillet_all_edges(*box, 2.0);
    require(rounded && rounded->ok(), "all-edge fillet failed");
    require(rounded->volume_mm3() < box->volume_mm3(), "fillet did not remove material");
    require(rounded->volume_mm3() > 0.0, "fillet produced an empty solid");
    auto invalid = forma::fillet_all_edges(*box, 0.0);
    require(invalid && !invalid->ok() && invalid->error_code() == "INVALID_DIMENSION",
            "zero radius was accepted");
    auto oversized = forma::fillet_all_edges(*box, 100.0);
    require(oversized && !oversized->ok(), "oversized fillet was accepted");
    auto one_edge = forma::fillet_edge(*box, "box-edge:x:ymin:zmin", 2.0);
    require(one_edge && one_edge->ok(), "semantic edge fillet failed");
    require(one_edge->volume_mm3() < box->volume_mm3() &&
            one_edge->volume_mm3() > rounded->volume_mm3(),
            "selected-edge fillet changed the wrong amount of material");
    auto resized = forma::make_box(80.0, 45.0, 12.0);
    auto resized_edge = forma::fillet_edge(*resized, "box-edge:x:ymin:zmin", 2.0);
    require(resized_edge && resized_edge->ok(), "edge key did not survive dimension change");
    auto missing = forma::fillet_edge(*box, "box-edge:x:ymin:zmiddle", 2.0);
    require(missing && !missing->ok() && missing->error_code() == "EDGE_REFERENCE_UNRESOLVED",
            "missing edge selector chose another edge");
  });
}
