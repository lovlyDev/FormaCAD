#pragma once

#include <TopoDS_Face.hxx>

namespace forma {

// Surface area comes from BREP geometry, not the tessellated viewport mesh.
double face_area_mm2(const TopoDS_Face& face);

} // namespace forma
