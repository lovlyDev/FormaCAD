#pragma once
#include <cstddef>
#include <TopoDS_Face.hxx>

namespace forma {
// Throws inside the exception boundary. Loops are canonical outer CCW/holes CW.
TopoDS_Face profile_face(const double* coordinates, std::size_t coordinate_count,
    const std::size_t* offsets, std::size_t offset_count, int plane,
    double origin_x_mm, double origin_y_mm, double origin_z_mm);
} // namespace forma
