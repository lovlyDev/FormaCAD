#pragma once
#include "forma_core.hpp"
namespace forma {
// Coordinates follow the underlying BREP edge's first/last vertices, never a
// world-coordinate sort. Rigid transforms therefore retain the radius mapping.
class EdgeEndpoints final {
public:
  bool valid() const noexcept { return code_.empty(); }
  const std::vector<double>& start_mm() const noexcept { return start_; }
  const std::vector<double>& end_mm() const noexcept { return end_; }
  const std::string& error_code() const noexcept { return code_; }
  const std::string& error_message() const noexcept { return detail_; }
private:
  std::vector<double> start_, end_;
  std::string code_, detail_;
  friend std::unique_ptr<EdgeEndpoints> variable_fillet_endpoints(const Shape&, const std::string&, const std::string&, const std::string&) noexcept;
};
std::unique_ptr<EdgeEndpoints> variable_fillet_endpoints(const Shape&, const std::string&, const std::string&, const std::string&) noexcept;
std::unique_ptr<Shape> variable_fillet(const Shape&, const std::string&, const std::string&, const std::string&, double, double) noexcept;
}
