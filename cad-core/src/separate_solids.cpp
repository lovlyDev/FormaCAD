#include "internal/shape_impl.hpp"

#include <NCollection_IndexedMap.hxx>
#include <Standard_Failure.hxx>
#include <TopExp.hxx>
#include <TopTools_ShapeMapHasher.hxx>

namespace forma {
namespace {
using SolidMap = NCollection_IndexedMap<TopoDS_Shape, TopTools_ShapeMapHasher>;
}

int Shape::solid_count() const noexcept {
  if (!ok()) return 0;
  try {
    SolidMap solids;
    TopExp::MapShapes(impl_->shape, TopAbs_SOLID, solids);
    return solids.Extent();
  } catch (...) {
    return 0;
  }
}

std::unique_ptr<Shape> extract_solid(const Shape& source, int index, int expected_count) noexcept {
  if (!source.ok() || expected_count < 2 || expected_count > 128 || index < 0 || index >= expected_count)
    return failure("SEPARATE_SOLIDS_FAILED", "Invalid solid selection");
  try {
    SolidMap solids;
    TopExp::MapShapes(source.impl_->shape, TopAbs_SOLID, solids);
    if (solids.Extent() != expected_count)
      return failure("SEPARATE_SOLIDS_FAILED", "The source solid count changed");
    return checked(solids(index + 1));
  } catch (const Standard_Failure& error) {
    return failure("SEPARATE_SOLIDS_FAILED", error.GetMessageString());
  } catch (...) {
    return failure("SEPARATE_SOLIDS_FAILED", "Solid extraction failed");
  }
}
}
