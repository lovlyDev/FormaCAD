#include "internal/shape_impl.hpp"
#include "internal/transform_axis.hpp"
#include <BRepAdaptor_Curve.hxx>
#include <BRepAlgoAPI_Section.hxx>
#include <BRepGProp.hxx>
#include <GProp_GProps.hxx>
#include <GeomAbs_Shape.hxx>
#include <NCollection_IndexedMap.hxx>
#include <Standard_Failure.hxx>
#include <NCollection_Array1.hxx>
#include <TopExp.hxx>
#include <TopTools_ShapeMapHasher.hxx>
#include <TopoDS.hxx>
#include <gp_Dir.hxx>
#include <gp_Pln.hxx>
#include <gp_Pnt.hxx>
#include <algorithm>
#include <array>
#include <cmath>
#include <stdexcept>

namespace forma {
namespace {
constexpr std::size_t max_edges=4096,max_points=100000;
struct SectionLimit : std::runtime_error {using std::runtime_error::runtime_error;};
void append(std::vector<double>& points,const gp_Pnt& point) {
  if (points.size()/3>=max_points) throw SectionLimit("Section preview point budget exceeded");
  if (!std::isfinite(point.X()) || !std::isfinite(point.Y()) || !std::isfinite(point.Z()))
    throw std::runtime_error("Section curve contains nonfinite points");
  points.insert(points.end(),{point.X(),point.Y(),point.Z()});
}
// A bounded visual approximation, separate from integrated BREP curve length.
void sample(const BRepAdaptor_Curve& curve,double first,double last,const gp_Pnt& a,const gp_Pnt& b,
            double deflection,int depth,std::vector<double>& points) {
  double deviation=0;
  const double middle=(first+last)/2;
  gp_Pnt midpoint;
  for (double fraction : {0.25,0.5,0.75}) {
    const gp_Pnt p=curve.Value(first+(last-first)*fraction);
    const gp_Pnt chord(a.X()+(b.X()-a.X())*fraction,a.Y()+(b.Y()-a.Y())*fraction,a.Z()+(b.Z()-a.Z())*fraction);
    deviation=std::max(deviation,p.Distance(chord));
    if (fraction==0.5) midpoint=p;
  }
  if (!std::isfinite(deviation)) throw std::runtime_error("Section curve deviation is invalid");
  if (deviation<=deflection) {append(points,b);return;}
  if (depth>=24 || middle==first || middle==last) throw SectionLimit("Section approximation cannot meet its bounded sampling budget");
  sample(curve,first,middle,a,midpoint,deflection,depth+1,points);
  sample(curve,middle,last,midpoint,b,deflection,depth+1,points);
}
std::unique_ptr<SectionResult> failed(const char* code,const char* detail) {
  auto result=std::make_unique<SectionResult>();result->code=code;result->detail=detail;return result;
}
}
std::unique_ptr<SectionResult> section_plane(const Shape& source,double ox,double oy,double oz,
    double nx,double ny,double nz,double deflection) noexcept {
  if (!valid_transform_axis(ox,oy,oz,nx,ny,nz) || !std::isfinite(deflection) || deflection<0.001 || deflection>1)
    return failed("INVALID_SECTION_PLANE","Section origin, normal and deflection must be finite and bounded");
  if (!source.ok()) return failed("INVALID_SOURCE","Section requires a valid BREP source");
  try {
    BRepAlgoAPI_Section builder(source.impl_->shape,gp_Pln(gp_Pnt(ox,oy,oz),gp_Dir(nx,ny,nz)),false);
    builder.Approximation(false);builder.Build();
    if (!builder.IsDone()) return failed("SECTION_FAILED","BREP plane intersection failed");
    auto result=std::make_unique<SectionResult>();
    NCollection_IndexedMap<TopoDS_Shape,TopTools_ShapeMapHasher> edges;
    TopExp::MapShapes(builder.Shape(),TopAbs_EDGE,edges);
    if (edges.Extent()>static_cast<int>(max_edges)) throw SectionLimit("Section edge budget exceeded");
    for (int index=1;index<=edges.Extent();++index) {
      const auto edge=TopoDS::Edge(edges.FindKey(index));
      GProp_GProps properties;BRepGProp::LinearProperties(edge,properties);
      const double length=properties.Mass();
      if (!std::isfinite(length) || length<0) throw std::runtime_error("Section curve length is invalid");
      if (length<=1e-9) continue;
      BRepAdaptor_Curve curve(edge);
      const int intervals=curve.NbIntervals(GeomAbs_C1);
      if (intervals<=0 || intervals>4096) throw SectionLimit("Section curve continuity budget exceeded");
      NCollection_Array1<double> bounds(1,intervals+1);curve.Intervals(bounds,GeomAbs_C1);
      const double first=bounds(1);const double last=bounds(intervals+1);
      if (!std::isfinite(first)||!std::isfinite(last)||last<=first) throw std::runtime_error("Section curve range is invalid");
      append(result->points,curve.Value(first));
      for (int interval=1;interval<=intervals;++interval) {
        const double u0=bounds(interval),u1=bounds(interval+1);
        sample(curve,u0,u1,curve.Value(u0),curve.Value(u1),deflection,0,result->points);
      }
      result->lengths.push_back(length);result->offsets.push_back(static_cast<std::uint32_t>(result->points.size()/3));
    }
    return result;
  } catch (const SectionLimit& e) {return failed("SECTION_LIMIT",e.what());}
    catch (const Standard_Failure& e) {return failed("SECTION_FAILED",e.GetMessageString());}
    catch (const std::exception& e) {return failed("SECTION_FAILED",e.what());}
    catch (...) {return failed("SECTION_FAILED","Unexpected BREP section failure");}
}
} // namespace forma
