#include "forma_core.hpp"
#include "test_support.hpp"
#include <array>
#include <cmath>
#include <limits>

static void point(const forma::SnapResult& result, const std::array<double, 3>& expected) {
  require(result.valid(), result.error_message().c_str());
  require(result.point_mm().size() == 3, "Snap point dimension");
  for (std::size_t axis = 0; axis < 3; ++axis)
    require(std::abs(result.point_mm()[axis] - expected[axis]) < 1e-6, "Exact snap point oracle");
}
int main() {
  return run_test([] {
    auto box = forma::make_referenced_box(20, 10, 4, "pad");
    auto midpoint = forma::snap_reference(*box, 0, "pad", "box-edge:x:ymin:zmax", "", nullptr, 0);
    point(*midpoint, {0, -5, 4});
    auto rotated = forma::rotate(*box, 0, 0, 0, 0, 0, 1, 90);
    rotated = forma::topology_occurrence(*rotated, "spin");
    auto moved = forma::translate(*rotated, 10, 20, 30);
    moved = forma::topology_occurrence(*moved, "offset");
    auto transformed_midpoint = forma::snap_reference(*moved, 0, "pad", "box-edge:x:ymin:zmax", "spin/offset", nullptr, 0);
    point(*transformed_midpoint, {15, 20, 34});
    require(!forma::snap_reference(*moved, 0, "pad", "box-edge:x:ymin:zmax", "wrong", nullptr, 0)->valid(), "Reject wrong semantic occurrence");
    require(std::abs(box->volume_mm3() - 800.) < 1e-8, "Read-only source invariant");

    auto cylinder = forma::make_referenced_cylinder(5, 12, "cylinder");
    auto center = forma::snap_reference(*cylinder, 1, "cylinder", "cylinder-edge:top", "", nullptr, 0);
    point(*center, {0, 0, 12});
    const std::array<double, 12> stretch{2,0,0,0, 0,1,0,0, 0,0,1,0};
    auto ellipse = forma::transform_affine(*cylinder, stretch.data(), stretch.size());
    ellipse = forma::topology_occurrence(*ellipse, "stretch");
    point(*forma::snap_reference(*ellipse, 0, "cylinder", "cylinder-edge:top", "stretch", nullptr, 0), {-10, 0, 12});
    require(!forma::snap_reference(*ellipse, 1, "cylinder", "cylinder-edge:top", "stretch", nullptr, 0)->valid(), "Ellipse must not be called a circle");
    auto turned = forma::rotate(*cylinder, 0, 0, 0, 0, 1, 0, 90);
    turned = forma::topology_occurrence(*turned, "spin");
    auto offset = forma::translate(*turned, -7, 11, 13);
    offset = forma::topology_occurrence(*offset, "offset");
    point(*forma::snap_reference(*offset, 1, "cylinder", "cylinder-edge:top", "spin/offset", nullptr, 0), {5, 11, 13});
    require(!forma::snap_reference(*box, 1, "pad", "box-edge:x:ymin:zmax", "", nullptr, 0)->valid(), "Straight edge is not a circle");

    const std::array<double, 6> ray{2, 3, 50, 0, 0, -1};
    auto face = forma::snap_reference(*box, 2, "pad", "box-face:zmax", "", ray.data(), ray.size());
    point(*face, {2, 3, 4});
    require(face->normal().size() == 3 && std::abs(face->normal()[2] - 1.) < 1e-8, "Outward face normal");
    const std::array<double, 6> curved_ray{-20, 0, 6, 1, 0, 0};
    auto curved = forma::snap_reference(*cylinder, 2, "cylinder", "cylinder-face:side", "", curved_ray.data(), curved_ray.size());
    point(*curved, {-5, 0, 6});
    require(std::abs(curved->normal()[0] + 1.) < 1e-8, "Nearest curved-face hit and outward normal");
    auto mirrored = forma::mirror(*box, 0, 0, 0, 1, 0, 0);
    mirrored = forma::topology_occurrence(*mirrored, "mirror");
    const std::array<double, 6> mirror_ray{50, 0, 2, -1, 0, 0};
    auto mirrored_face = forma::snap_reference(*mirrored, 2, "pad", "box-face:xmin", "mirror", mirror_ray.data(), mirror_ray.size());
    point(*mirrored_face, {10, 0, 2});
    require(std::abs(mirrored_face->normal()[0] - 1.) < 1e-8, "Mirror preserves outward normal orientation");
    auto miss = ray; miss[0] = 99;
    require(!forma::snap_reference(*box, 2, "pad", "box-face:zmax", "", miss.data(), miss.size())->valid(), "Respect trimmed face boundary");
    miss = ray; miss[5] = 1;
    require(!forma::snap_reference(*box, 2, "pad", "box-face:zmax", "", miss.data(), miss.size())->valid(), "No backward ray hit");
    const std::array<double, 12> shear{1,0,.5,10, 0,2,0,-5, 0,0,3,7};
    auto sheared = forma::transform_affine(*box, shear.data(), shear.size());
    sheared = forma::topology_occurrence(*sheared, "affine");
    const std::array<double, 6> side_ray{-50, -5, 13, 1, 0, 0};
    auto side = forma::snap_reference(*sheared, 2, "pad", "box-face:xmin", "affine", side_ray.data(), side_ray.size());
    point(*side, {1, -5, 13});
    require(std::abs(side->normal()[0] + 6 / std::sqrt(37.)) < 1e-7 && std::abs(side->normal()[2] - 1 / std::sqrt(37.)) < 1e-7, "Affine outward normal oracle");

    auto bad = ray; bad[3] = std::numeric_limits<double>::quiet_NaN();
    require(!forma::snap_reference(*box, 2, "pad", "box-face:zmax", "", bad.data(), bad.size())->valid(), "Reject NaN ray");
    bad = ray; bad[5] = 0;
    require(!forma::snap_reference(*box, 2, "pad", "box-face:zmax", "", bad.data(), bad.size())->valid(), "Reject zero ray");
    require(!forma::snap_reference(*box, 2, "pad", "box-face:zmax", "", nullptr, 0)->valid(), "Require face ray");
    require(!forma::snap_reference(*box, 99, "pad", "box-edge:x:ymin:zmax", "", nullptr, 0)->valid(), "Reject unknown query");
  });
}
