#pragma once
#include <memory>
#include <string>
#include <vector>

namespace forma {
class Shape;
// Stable FFI query tags: body=0, length=1, radius=2, area=3, planar face=4, diameter=5.
class MeasurementResult final {
public:
  bool valid() const noexcept { return code_.empty(); }
  int query_kind() const noexcept { return kind_; }
  double length_mm() const noexcept { return length_; }
  double radius_mm() const noexcept { return radius_; }
  double diameter_mm() const noexcept { return diameter_; }
  double area_mm2() const noexcept { return area_; }
  double volume_mm3() const noexcept { return volume_; }
  int face_count() const noexcept { return faces_; }
  int edge_count() const noexcept { return edges_; }
  const std::vector<double>& extents_mm() const noexcept { return extents_; }
  const std::vector<double>& origin_mm() const noexcept { return origin_; }
  const std::vector<double>& normal() const noexcept { return normal_; }
  const std::string& error_code() const noexcept { return code_; }
  const std::string& error_message() const noexcept { return detail_; }
private:
  int kind_ = -1, faces_ = 0, edges_ = 0;
  double length_ = 0, radius_ = 0, diameter_ = 0, area_ = 0, volume_ = 0;
  std::vector<double> extents_, origin_, normal_;
  std::string code_, detail_;
  friend std::unique_ptr<MeasurementResult> measure_reference(const Shape&, int,
    const std::string&, const std::string&, const std::string&) noexcept;
};
std::unique_ptr<MeasurementResult> measure_reference(const Shape&, int query_kind,
  const std::string& owner, const std::string& role, const std::string& occurrence_path) noexcept;
}
