#pragma once
#include <memory>
#include <string>
#include <vector>

namespace forma {
class Shape;
// Separate FFI namespace: distance=0, outward face-normal angle=1,
// unoriented straight-edge acute angle=2. Existing single-query tags unchanged.
class PairMeasurementResult final {
public:
  bool valid() const noexcept { return code_.empty(); }
  int query_kind() const noexcept { return kind_; }
  double distance_mm() const noexcept { return distance_; }
  double angle_deg() const noexcept { return angle_; }
  const std::vector<double>& point_a_mm() const noexcept { return point_a_; }
  const std::vector<double>& point_b_mm() const noexcept { return point_b_; }
  const std::string& error_code() const noexcept { return code_; }
  const std::string& error_message() const noexcept { return detail_; }
private:
  int kind_ = -1;
  double distance_ = 0, angle_ = 0;
  std::vector<double> point_a_, point_b_;
  std::string code_, detail_;
  friend std::unique_ptr<PairMeasurementResult> measure_reference_pair(const Shape&, int,
    int, const std::string&, const std::string&, const std::string&,
    int, const std::string&, const std::string&, const std::string&) noexcept;
};
// Entity kind: edge=0, face=1; both belong to the same selected source body.
std::unique_ptr<PairMeasurementResult> measure_reference_pair(const Shape&, int query_kind,
  int kind_a, const std::string& owner_a, const std::string& role_a, const std::string& path_a,
  int kind_b, const std::string& owner_b, const std::string& role_b, const std::string& path_b) noexcept;
}
