#include "internal/shape_impl.hpp"

#include <BRepCheck_Analyzer.hxx>
#include <BRepGProp.hxx>
#include <GProp_GProps.hxx>
#include <NCollection_IndexedMap.hxx>
#include <Standard_Failure.hxx>
#include <TopAbs_ShapeEnum.hxx>
#include <TopExp.hxx>
#include <TopTools_ShapeMapHasher.hxx>

#include <cmath>
#include <exception>
#include <utility>

namespace forma {

Shape::Shape() : impl_(std::make_unique<Impl>()) {}
Shape::~Shape() = default;
Shape::Shape(Shape&&) noexcept = default;
Shape& Shape::operator=(Shape&&) noexcept = default;
bool Shape::ok() const noexcept { return impl_ && impl_->code.empty(); }
double Shape::volume_mm3() const noexcept { return ok() ? impl_->volume : 0.0; }
double Shape::surface_area_mm2() const noexcept { return ok() ? impl_->area : 0.0; }
int Shape::face_count() const noexcept { return ok() ? impl_->faces : 0; }
int Shape::edge_count() const noexcept { return ok() ? impl_->edges : 0; }
const std::string& Shape::error_code() const noexcept {
  static const std::string missing = "MISSING_SHAPE";
  return impl_ ? impl_->code : missing;
}
const std::string& Shape::error_message() const noexcept {
  static const std::string missing = "Shape handle is missing";
  return impl_ ? impl_->message : missing;
}

std::unique_ptr<Shape> failure(const char* code, const char* message) noexcept {
  try {
    auto result = std::make_unique<Shape>();
    result->impl_->code = code;
    result->impl_->message = message;
    return result;
  } catch (...) {
    return nullptr;
  }
}

std::unique_ptr<Shape> checked(TopoDS_Shape shape) noexcept {
  try {
    if (shape.IsNull()) return failure("EMPTY_SHAPE", "Kernel returned an empty shape");
    if (!BRepCheck_Analyzer(shape).IsValid())
      return failure("INVALID_TOPOLOGY", "Kernel returned invalid topology");
    GProp_GProps props;
    BRepGProp::VolumeProperties(shape, props);
    const double volume = props.Mass();
    if (!std::isfinite(volume) || volume <= 0.0)
      return failure("INVALID_VOLUME", "Solid volume must be finite and positive");
    GProp_GProps area_props;
    BRepGProp::SurfaceProperties(shape, area_props);
    const double area = area_props.Mass();
    if (!std::isfinite(area) || area <= 0.0)
      return failure("INVALID_AREA", "Solid surface area must be finite and positive");
    auto result = std::make_unique<Shape>();
    result->impl_->shape = std::move(shape);
    result->impl_->volume = volume;
    result->impl_->area = area;
    NCollection_IndexedMap<TopoDS_Shape, TopTools_ShapeMapHasher> faces;
    NCollection_IndexedMap<TopoDS_Shape, TopTools_ShapeMapHasher> edges;
    TopExp::MapShapes(result->impl_->shape, TopAbs_FACE, faces);
    TopExp::MapShapes(result->impl_->shape, TopAbs_EDGE, edges);
    result->impl_->faces = faces.Extent();
    result->impl_->edges = edges.Extent();
    if (!result->update_bounds())
      return failure("INVALID_BOUNDS", "Solid bounding dimensions are invalid");
    return result;
  } catch (const Standard_Failure& error) {
    return failure("OCCT_ERROR", error.GetMessageString());
  } catch (const std::exception& error) {
    return failure("NATIVE_ERROR", error.what());
  } catch (...) {
    return failure("NATIVE_ERROR", "Unknown native error");
  }
}

} // namespace forma
