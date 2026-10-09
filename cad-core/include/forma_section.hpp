#pragma once
#include <cstdint>
#include <memory>
#include <string>
#include <vector>

namespace forma {
class Shape;
class SectionResult final {
public:
  bool valid() const noexcept { return code.empty(); }
  const std::string& error_code() const noexcept { return code; }
  const std::string& error_message() const noexcept { return detail; }
  const std::vector<double>& points_mm() const noexcept { return points; }
  const std::vector<std::uint32_t>& edge_offsets() const noexcept { return offsets; }
  const std::vector<double>& edge_lengths_mm() const noexcept { return lengths; }
  std::string code;
  std::string detail;
  std::vector<double> points;
  std::vector<std::uint32_t> offsets{0};
  std::vector<double> lengths;
};
std::unique_ptr<SectionResult> section_plane(const Shape&,double,double,double,double,double,double,double) noexcept;
} // namespace forma
