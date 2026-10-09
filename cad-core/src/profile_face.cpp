#include "internal/profile_face.hpp"
#include <BRepBuilderAPI_MakeFace.hxx>
#include <BRepBuilderAPI_MakePolygon.hxx>
#include <BRepCheck_Analyzer.hxx>
#include <gp_Pln.hxx>
#include <gp_Pnt.hxx>
#include <gp_Dir.hxx>
#include <cmath>
#include <memory>
#include <stdexcept>

namespace forma {
TopoDS_Face profile_face(const double* coordinates, std::size_t coordinate_count,
    const std::size_t* offsets, std::size_t offset_count, int plane,
    double ox, double oy, double oz) {
  if (!coordinates || !offsets || coordinate_count < 6 || coordinate_count > 64 || coordinate_count % 2 ||
      offset_count < 2 || offset_count > 9 || offsets[0] != 0 || offsets[offset_count-1] != coordinate_count ||
      plane < 0 || plane > 2 || !std::isfinite(ox) || !std::isfinite(oy) || !std::isfinite(oz) ||
      std::abs(ox)>10000 || std::abs(oy)>10000 || std::abs(oz)>10000)
    throw std::invalid_argument("Sketch profile layout is invalid");
  for (std::size_t i=1;i<offset_count;++i)
    if (offsets[i]>coordinate_count || offsets[i]%2 || offsets[i]<=offsets[i-1] || offsets[i]-offsets[i-1]<6)
      throw std::invalid_argument("Sketch profile layout is invalid");
  const gp_Dir normal = plane==0 ? gp_Dir(0,0,1) : plane==1 ? gp_Dir(0,-1,0) : gp_Dir(1,0,0);
  const gp_Pln workplane(gp_Pnt(ox,oy,oz), normal);
  std::unique_ptr<BRepBuilderAPI_MakeFace> face;
  for (std::size_t loop=0;loop+1<offset_count;++loop) {
    BRepBuilderAPI_MakePolygon polygon;
    for (std::size_t i=offsets[loop];i<offsets[loop+1];i+=2) {
      const double u=coordinates[i],v=coordinates[i+1];
      if (!std::isfinite(u)||!std::isfinite(v)||std::abs(u)>10000||std::abs(v)>10000)
        throw std::invalid_argument("Polygon coordinates must be finite and bounded");
      polygon.Add(plane==0 ? gp_Pnt(ox+u,oy+v,oz) : plane==1 ? gp_Pnt(ox+u,oy,oz+v) : gp_Pnt(ox,oy+u,oz+v));
    }
    polygon.Close();
    if (!polygon.IsDone()) throw std::invalid_argument("Polygon outline is not closed");
    if (loop==0) {
      face = std::make_unique<BRepBuilderAPI_MakeFace>(workplane,polygon.Wire(),true);
      if (!face->IsDone()) throw std::invalid_argument("Sketch profile face is invalid");
    } else face->Add(polygon.Wire());
  }
  if (!face || !face->IsDone() || !BRepCheck_Analyzer(face->Face()).IsValid())
    throw std::invalid_argument("Sketch profile face is invalid");
  return face->Face();
}
} // namespace forma
