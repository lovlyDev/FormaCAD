#include "forma_offset_profile.hpp"
#include "internal/profile_face.hpp"
#include "internal/transform_axis.hpp"
#include <BRepAdaptor_Surface.hxx>
#include <BRepAlgoAPI_Common.hxx>
#include <BRepAlgoAPI_Cut.hxx>
#include <BRepAlgo_FaceRestrictor.hxx>
#include <BRepBuilderAPI_Copy.hxx>
#include <BRepBuilderAPI_MakeEdge.hxx>
#include <BRepBuilderAPI_MakeFace.hxx>
#include <BRepBuilderAPI_MakeWire.hxx>
#include <BRepCheck_Analyzer.hxx>
#include <BRepGProp.hxx>
#include <BRepOffsetAPI_MakeOffset.hxx>
#include <BRepPrimAPI_MakePrism.hxx>
#include <BRepPrimAPI_MakeRevol.hxx>
#include <BRepTools.hxx>
#include <GProp_GProps.hxx>
#include <NCollection_List.hxx>
#include <Precision.hxx>
#include <TopExp_Explorer.hxx>
#include <TopoDS.hxx>
#include <gp_Circ.hxx>
#include <gp_Pln.hxx>
#include <gp_Vec.hxx>
#include <algorithm>
#include <cmath>
#include <stdexcept>

namespace forma {
struct PlanarProfile::Impl {
  TopoDS_Face face;
  int plane = 0, contours = 0;
  double area = 0, perimeter = 0;
  std::string code = "INVALID_PROFILE", detail = "Profile is not initialized";
};
PlanarProfile::PlanarProfile():impl_(std::make_unique<Impl>()){}
PlanarProfile::~PlanarProfile() = default;
bool PlanarProfile::valid() const noexcept {return impl_ && impl_->code.empty();}
double PlanarProfile::area_mm2() const noexcept {return valid()?impl_->area:0;}
double PlanarProfile::perimeter_mm() const noexcept {return valid()?impl_->perimeter:0;}
int PlanarProfile::contour_count() const noexcept {return valid()?impl_->contours:0;}
const std::string& PlanarProfile::error_code() const noexcept {return impl_->code;}
const std::string& PlanarProfile::error_message() const noexcept {return impl_->detail;}
std::unique_ptr<PlanarProfile> profile_failure(const char* code,const char* detail) {
  auto out=std::make_unique<PlanarProfile>();out->impl_->code=code;out->impl_->detail=detail;return out;
}
namespace {
int count(const TopoDS_Shape& shape,TopAbs_ShapeEnum type){int n=0;for(TopExp_Explorer e(shape,type);e.More();e.Next())++n;return n;}
double area(const TopoDS_Shape& shape){GProp_GProps p;BRepGProp::SurfaceProperties(shape,p);return p.Mass();}
gp_Vec normal(int plane){return plane==0?gp_Vec(0,0,1):plane==1?gp_Vec(0,-1,0):gp_Vec(1,0,0);}
template<class Operation> TopoDS_Shape boolean_shape(const TopoDS_Shape& a,const TopoDS_Shape& b){
  NCollection_List<TopoDS_Shape> args,tools;args.Append(a);tools.Append(b);
  Operation op;op.SetArguments(args);op.SetTools(tools);op.SetNonDestructive(true);op.Build();
  if(!op.IsDone())throw std::runtime_error("Profile boolean failed");return op.Shape();
}
TopoDS_Face single_face(const TopoDS_Shape& shape){
  if(shape.IsNull()||count(shape,TopAbs_FACE)!=1)throw std::invalid_argument("Offset must retain one connected profile");
  return TopoDS::Face(TopExp_Explorer(shape,TopAbs_FACE).Current());
}
TopoDS_Face offset_loop(const TopoDS_Face& face,double distance,bool rounded){
  BRepOffsetAPI_MakeOffset op(face,rounded?GeomAbs_Arc:GeomAbs_Intersection,false);op.Perform(distance);
  if(!op.IsDone()||op.Shape().IsNull()||count(op.Shape(),TopAbs_WIRE)!=1)
    throw std::invalid_argument("Offset collapses or splits a contour");
  BRepAlgo_FaceRestrictor restrictor;restrictor.Init(face,true,true);
  auto wire=TopoDS::Wire(TopExp_Explorer(op.Shape(),TopAbs_WIRE).Current());restrictor.Add(wire);restrictor.Perform();
  if(!restrictor.IsDone()||!restrictor.More())throw std::invalid_argument("Offset has no finite face");
  auto result=restrictor.Current();restrictor.Next();
  if(restrictor.More()||!BRepCheck_Analyzer(result).IsValid())throw std::invalid_argument("Offset splits a contour");
  return result;
}
}
std::unique_ptr<PlanarProfile> profile_checked(TopoDS_Face face,int plane){
  if(face.IsNull()||!BRepCheck_Analyzer(face).IsValid()||BRepAdaptor_Surface(face).GetType()!=GeomAbs_Plane)
    return profile_failure("INVALID_PROFILE","Profile BREP is invalid");
  GProp_GProps length;BRepGProp::LinearProperties(face,length);
  const auto a=area(face),p=length.Mass();
  if(!std::isfinite(a)||a<=0||!std::isfinite(p)||p<=0)
    return profile_failure("INVALID_PROFILE","Profile measurements must be finite and positive");
  auto out=std::make_unique<PlanarProfile>();out->impl_->face=face;out->impl_->plane=plane;
  out->impl_->area=a;out->impl_->perimeter=p;out->impl_->contours=count(face,TopAbs_WIRE);
  out->impl_->code.clear();out->impl_->detail.clear();return out;
}
std::unique_ptr<PlanarProfile> make_planar_profile(const double* c,std::size_t n,const std::size_t* offsets,std::size_t loops,int plane,double x,double y,double z) noexcept {
  try{return profile_checked(profile_face(c,n,offsets,loops,plane,x,y,z),plane);}
  catch(const std::exception& e){return profile_failure("INVALID_PROFILE",e.what());}
  catch(const Standard_Failure& e){return profile_failure("INVALID_PROFILE",e.GetMessageString());}
  catch(...){return profile_failure("INVALID_PROFILE","Profile construction failed");}
}
std::unique_ptr<PlanarProfile> make_circle_profile(double radius) noexcept {
  if(!std::isfinite(radius)||radius<=Precision::Confusion()||radius>10000)
    return profile_failure("INVALID_PROFILE","Circle radius is invalid");
  try {
    const gp_Pln plane(gp_Pnt(0,0,0),gp_Dir(0,0,1));
    auto wire=BRepBuilderAPI_MakeWire(BRepBuilderAPI_MakeEdge(gp_Circ(gp_Ax2(gp_Pnt(0,0,0),gp_Dir(0,0,1)),radius)).Edge()).Wire();
    return profile_checked(BRepBuilderAPI_MakeFace(plane,wire,true).Face(),0);
  } catch(const Standard_Failure& e){return profile_failure("INVALID_PROFILE",e.GetMessageString());}
    catch(...){return profile_failure("INVALID_PROFILE","Circle profile construction failed");}
}
std::unique_ptr<PlanarProfile> offset_profile(const PlanarProfile& source,double distance,bool rounded) noexcept {
  if(!source.valid())return profile_failure("INVALID_PROFILE","Offset source is invalid");
  if(!std::isfinite(distance)||std::abs(distance)<=Precision::Confusion()||std::abs(distance)>10000)
    return profile_failure("INVALID_PROFILE_OFFSET","Profile offset must be finite and nonzero");
  try {
    // Copy all geometry before any OCCT routine can attach pcurves to wires.
    auto face=TopoDS::Face(BRepBuilderAPI_Copy(source.impl_->face,true,false).Shape());
    const auto plane=BRepAdaptor_Surface(face).Plane();
    const auto outer=BRepTools::OuterWire(face);
    TopoDS_Shape result=offset_loop(BRepBuilderAPI_MakeFace(plane,outer,true).Face(),distance,rounded);
    for(TopExp_Explorer e(face,TopAbs_WIRE);e.More();e.Next()) {
      const auto wire=TopoDS::Wire(e.Current());if(wire.IsSame(outer))continue;
      const auto hole=offset_loop(BRepBuilderAPI_MakeFace(plane,wire,true).Face(),-distance,rounded);
      const auto hole_area=area(hole);
      const auto inside=boolean_shape<BRepAlgoAPI_Common>(result,hole);
      if(std::abs(area(inside)-hole_area)>std::max(1e-7,hole_area*1e-9))
        throw std::invalid_argument("Offset holes overlap or leave the outer contour");
      result=boolean_shape<BRepAlgoAPI_Cut>(result,hole);
    }
    const auto out_face=single_face(result);
    if(count(out_face,TopAbs_WIRE)!=source.impl_->contours)
      throw std::invalid_argument("Offset changes the number of contours");
    return profile_checked(out_face,source.impl_->plane);
  } catch(const std::invalid_argument& e){return profile_failure("PROFILE_OFFSET_TOPOLOGY_CHANGED",e.what());}
    catch(const Standard_Failure& e){return profile_failure("PROFILE_OFFSET_FAILED",e.GetMessageString());}
    catch(const std::exception& e){return profile_failure("PROFILE_OFFSET_FAILED",e.what());}
    catch(...){return profile_failure("PROFILE_OFFSET_FAILED","Profile offset failed");}
}
std::unique_ptr<Shape> extrude_planar_profile(const PlanarProfile& source,double distance) noexcept {
  if(!source.valid())return failure("INVALID_PROFILE","Extrusion profile is invalid");
  if(!std::isfinite(distance)||distance==0||std::abs(distance)>10000)return failure("INVALID_DIMENSION","Extrusion distance is invalid");
  try{return checked(BRepPrimAPI_MakePrism(source.impl_->face,normal(source.impl_->plane)*distance,true).Shape());}
  catch(const Standard_Failure& e){return failure("PROFILE_EXTRUSION_FAILED",e.GetMessageString());}
  catch(...){return failure("PROFILE_EXTRUSION_FAILED","Profile extrusion failed");}
}
std::unique_ptr<Shape> revolve_planar_profile(const PlanarProfile& source,double x,double y,double z,double nx,double ny,double nz,double degrees) noexcept {
  if(!source.valid()||!valid_transform_axis(x,y,z,nx,ny,nz)||!std::isfinite(degrees)||degrees==0||std::abs(degrees)>360)
    return failure("INVALID_PROFILE","Profile revolution is invalid");
  try{return checked(BRepPrimAPI_MakeRevol(source.impl_->face,gp_Ax1(gp_Pnt(x,y,z),gp_Dir(nx,ny,nz)),degrees*std::acos(-1.)/180.,true).Shape());}
  catch(const Standard_Failure& e){return failure("PROFILE_REVOLUTION_FAILED",e.GetMessageString());}
  catch(...){return failure("PROFILE_REVOLUTION_FAILED","Profile revolution failed");}
}
}
