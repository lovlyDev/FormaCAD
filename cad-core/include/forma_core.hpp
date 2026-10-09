#pragma once

#include <memory>
#include <cstddef>
#include <string>
#include "forma_section.hpp"
#include "forma_section.hpp"
#include "forma_measurement.hpp"
#include "forma_pair_measurement.hpp"

class TopoDS_Shape;

namespace forma {

enum class BooleanKind;

// No OCCT exception or raw TopoDS_Shape crosses the application boundary.
class Shape final {
public:
  Shape();
  ~Shape();
  Shape(Shape&&) noexcept;
  Shape& operator=(Shape&&) noexcept;
  Shape(const Shape&) = delete;
  Shape& operator=(const Shape&) = delete;

  bool ok() const noexcept;
  double volume_mm3() const noexcept;
  double surface_area_mm2() const noexcept;
  int face_count() const noexcept;
  int edge_count() const noexcept;
  double extent_x_mm() const noexcept;
  double extent_y_mm() const noexcept;
  double extent_z_mm() const noexcept;
  const std::string& error_code() const noexcept;
  const std::string& error_message() const noexcept;

private:
  bool update_bounds();
  friend std::unique_ptr<Shape> failure(const char*, const char*) noexcept;
  friend std::unique_ptr<Shape> checked(TopoDS_Shape) noexcept;
  friend std::unique_ptr<Shape> boolean_apply(const Shape&, const Shape&, BooleanKind) noexcept;
  friend std::unique_ptr<Shape> make_box(double, double, double) noexcept;
  friend std::unique_ptr<Shape> make_referenced_box(double, double, double, const std::string&) noexcept;
  friend std::unique_ptr<Shape> make_referenced_cylinder(double, double, const std::string&) noexcept;
  friend std::unique_ptr<Shape> topology_occurrence(const Shape&, const std::string&) noexcept;
  friend std::unique_ptr<Shape> fillet_referenced_edge(const Shape&, const std::string&, const std::string&, const std::string&, double) noexcept;
  friend std::unique_ptr<MeasurementResult> measure_reference(const Shape&, int, const std::string&, const std::string&, const std::string&) noexcept;
  friend std::unique_ptr<PairMeasurementResult> measure_reference_pair(const Shape&, int,
    int, const std::string&, const std::string&, const std::string&,
    int, const std::string&, const std::string&, const std::string&) noexcept;
  friend std::unique_ptr<Shape> make_cylinder(double, double) noexcept;
  friend std::unique_ptr<Shape> make_sphere(double) noexcept;
  friend std::unique_ptr<Shape> make_cone(double, double, double) noexcept;
  friend std::unique_ptr<Shape> make_polygon_prism(const double*, std::size_t, int, double, double, double, double) noexcept;
  friend std::unique_ptr<Shape> fillet_all_edges(const Shape&, double) noexcept;
  friend std::unique_ptr<Shape> fillet_edge(const Shape&, const std::string&, double) noexcept;
  friend std::unique_ptr<Shape> chamfer_all_edges(const Shape&, double) noexcept;
  friend std::unique_ptr<Shape> translate(const Shape&, double, double, double) noexcept;
  friend std::unique_ptr<Shape> rotate(const Shape&, double, double, double, double, double, double, double) noexcept;
  friend std::unique_ptr<Shape> mirror(const Shape&, double, double, double, double, double, double) noexcept;
  friend std::unique_ptr<Shape> rotate(const Shape&, double, double, double, double, double, double, double) noexcept;
  friend std::unique_ptr<Shape> mirror(const Shape&, double, double, double, double, double, double) noexcept;
  friend std::unique_ptr<Shape> boolean_union(const Shape&, const Shape&) noexcept;
  friend std::unique_ptr<Shape> boolean_cut(const Shape&, const Shape&) noexcept;
  friend std::unique_ptr<Shape> boolean_intersect(const Shape&, const Shape&) noexcept;
  friend std::unique_ptr<Shape> make_compound(const Shape&, const Shape&) noexcept;
  friend std::unique_ptr<Shape> read_step(const std::string&) noexcept;
  friend bool write_step(const Shape&, const std::string&, std::string&) noexcept;
  friend bool write_glb(const Shape&, const std::string&, std::string&) noexcept;
  friend std::unique_ptr<SectionResult> section_plane(const Shape&,double,double,double,double,double,double,double) noexcept;
  friend std::unique_ptr<SectionResult> section_plane(const Shape&,double,double,double,double,double,double,double) noexcept;

  struct Impl;
  std::unique_ptr<Impl> impl_;
};

std::unique_ptr<Shape> failure(const char* code, const char* message) noexcept;
std::unique_ptr<Shape> checked(TopoDS_Shape shape) noexcept;
std::unique_ptr<Shape> make_box(double width_mm, double depth_mm, double height_mm) noexcept;
std::unique_ptr<Shape> make_referenced_box(double, double, double, const std::string&) noexcept;
std::unique_ptr<Shape> make_referenced_cylinder(double, double, const std::string&) noexcept;
std::unique_ptr<Shape> topology_occurrence(const Shape&, const std::string&) noexcept;
std::unique_ptr<Shape> fillet_referenced_edge(const Shape&, const std::string&, const std::string&, const std::string&, double) noexcept;
std::unique_ptr<Shape> make_cylinder(double radius_mm, double height_mm) noexcept;
std::unique_ptr<Shape> make_sphere(double radius_mm) noexcept;
std::unique_ptr<Shape> make_cone(double bottom_radius_mm, double top_radius_mm, double height_mm) noexcept;
std::unique_ptr<Shape> make_polygon_prism(const double* outline_xy, std::size_t coordinate_count, int plane,
                                          double origin_x_mm, double origin_y_mm,
                                          double origin_z_mm, double distance_mm) noexcept;
std::unique_ptr<Shape> make_profile_prism(const double* coordinates, std::size_t coordinate_count,
    const std::size_t* offsets, std::size_t offset_count, int plane,
    double origin_x_mm, double origin_y_mm, double origin_z_mm, double distance_mm) noexcept;
std::unique_ptr<Shape> fillet_all_edges(const Shape& source, double radius_mm) noexcept;
std::unique_ptr<Shape> make_profile_revolve(const double* coordinates, std::size_t coordinate_count,
    const std::size_t* offsets, std::size_t offset_count, int plane,
    double origin_x_mm, double origin_y_mm, double origin_z_mm,
    double axis_origin_x_mm, double axis_origin_y_mm, double axis_origin_z_mm,
    double axis_x, double axis_y, double axis_z, double angle_deg) noexcept;
std::unique_ptr<Shape> fillet_edge(const Shape& source, const std::string& edge_key, double radius_mm) noexcept;
std::unique_ptr<Shape> chamfer_all_edges(const Shape& source, double distance_mm) noexcept;
std::unique_ptr<Shape> translate(const Shape& source, double x_mm, double y_mm, double z_mm) noexcept;
std::unique_ptr<Shape> rotate(const Shape& source, double origin_x_mm, double origin_y_mm, double origin_z_mm,
    double axis_x, double axis_y, double axis_z, double angle_deg) noexcept;
std::unique_ptr<Shape> mirror(const Shape& source, double origin_x_mm, double origin_y_mm, double origin_z_mm,
    double normal_x, double normal_y, double normal_z) noexcept;
std::unique_ptr<Shape> rotate(const Shape& source, double origin_x_mm, double origin_y_mm, double origin_z_mm,
    double axis_x, double axis_y, double axis_z, double angle_deg) noexcept;
std::unique_ptr<Shape> mirror(const Shape& source, double origin_x_mm, double origin_y_mm, double origin_z_mm,
    double normal_x, double normal_y, double normal_z) noexcept;
std::unique_ptr<Shape> boolean_union(const Shape& left, const Shape& right) noexcept;
std::unique_ptr<Shape> boolean_cut(const Shape& left, const Shape& right) noexcept;
std::unique_ptr<Shape> boolean_intersect(const Shape& left, const Shape& right) noexcept;
std::unique_ptr<Shape> make_compound(const Shape& left, const Shape& right) noexcept;
std::unique_ptr<Shape> read_step(const std::string& path) noexcept;
bool write_step(const Shape& source, const std::string& path, std::string& error_code) noexcept;
bool write_glb(const Shape& source, const std::string& path, std::string& error_code) noexcept;

} // namespace forma
