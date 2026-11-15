#pragma once
#include "forma_core.hpp"
class TopoDS_Face;
namespace forma {
// A finite planar region, independent of the positive-volume Shape contract.
class PlanarProfile final {
public:
  PlanarProfile();
  ~PlanarProfile();
  bool valid() const noexcept;
  double area_mm2() const noexcept;
  double perimeter_mm() const noexcept;
  int contour_count() const noexcept;
  const std::string& error_code() const noexcept;
  const std::string& error_message() const noexcept;
private:
  struct Impl;
  std::unique_ptr<Impl> impl_;
  friend std::unique_ptr<PlanarProfile> profile_checked(TopoDS_Face, int);
  friend std::unique_ptr<PlanarProfile> profile_failure(const char*, const char*);
  friend std::unique_ptr<PlanarProfile> offset_profile(const PlanarProfile&, double, bool) noexcept;
  friend std::unique_ptr<Shape> extrude_planar_profile(const PlanarProfile&, double) noexcept;
  friend std::unique_ptr<Shape> revolve_planar_profile(const PlanarProfile&, double,double,double,double,double,double,double) noexcept;
};
std::unique_ptr<PlanarProfile> make_planar_profile(const double*,std::size_t,const std::size_t*,std::size_t,int,double,double,double) noexcept;
std::unique_ptr<PlanarProfile> make_circle_profile(double) noexcept;
std::unique_ptr<PlanarProfile> offset_profile(const PlanarProfile&,double,bool rounded) noexcept;
std::unique_ptr<Shape> extrude_planar_profile(const PlanarProfile&,double) noexcept;
std::unique_ptr<Shape> revolve_planar_profile(const PlanarProfile&,double,double,double,double,double,double,double) noexcept;
}
