#pragma once
#include <cstddef>
#include <memory>
#include <string>
#include <vector>

namespace forma {
class Shape;
class SnapResult final {
public:
  bool valid() const noexcept { return code_.empty(); }
  int query_kind() const noexcept { return kind_; }
  const std::vector<double>& point_mm() const noexcept { return point_; }
  const std::vector<double>& normal() const noexcept { return normal_; }
  const std::string& error_code() const noexcept { return code_; }
  const std::string& error_message() const noexcept { return detail_; }
private:
  int kind_ = -1;
  std::vector<double> point_, normal_;
  std::string code_, detail_;
  friend std::unique_ptr<SnapResult> snap_reference(const Shape&, int, const std::string&,
    const std::string&, const std::string&, const double*, std::size_t) noexcept;
};
// Tags: 0=edge arc-length midpoint, 1=analytic circle centre, 2=trimmed-face ray hit.
// Face ray is [origin x/y/z in mm, direction x/y/z]; other queries require zero ray values.
std::unique_ptr<SnapResult> snap_reference(const Shape&, int, const std::string&,
  const std::string&, const std::string&, const double*, std::size_t) noexcept;
}
