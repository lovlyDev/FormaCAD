#include "forma_iges.hpp"
#include "internal/shape_impl.hpp"
#include <BRepBuilderAPI_Copy.hxx>
#include <BRepLib.hxx>
#include <BRep_Builder.hxx>
#include <BRep_Tool.hxx>
#include <IGESControl_Reader.hxx>
#include <IGESControl_Writer.hxx>
#include <IFSelect_ReturnStatus.hxx>
#include <NCollection_IndexedMap.hxx>
#include <Standard_Failure.hxx>
#include <TopExp.hxx>
#include <TopExp_Explorer.hxx>
#include <TopoDS.hxx>
#include <TopoDS_Compound.hxx>
#include <TopoDS_Solid.hxx>
#include <TopTools_ShapeMapHasher.hxx>
#include <algorithm>
#include <cmath>
#include <stdexcept>

namespace forma {
namespace {
// No guessed sewing or first-solid fallback. All geometry must belong to closed
// BREP solids. Normalize orientation only, preserving cavities and components.
TopoDS_Shape closed_solids(const TopoDS_Shape& shape) {
  BRep_Builder builder;
  TopoDS_Compound result; builder.MakeCompound(result);
  int count=0;
  for (TopExp_Explorer it(shape,TopAbs_SOLID);it.More();it.Next()) {
    auto solid=TopoDS::Solid(it.Current());
    for (TopExp_Explorer shells(solid,TopAbs_SHELL);shells.More();shells.Next())
      if (!BRep_Tool::IsClosed(shells.Current())) throw std::runtime_error("IGES solid has an open shell");
    if (!BRepLib::OrientClosedSolid(solid)) throw std::runtime_error("IGES solid cannot be oriented");
    builder.Add(result,solid); ++count;
  }
  if (!count) throw std::runtime_error("IGES requires closed BREP solids; surface-only files are unsupported");
  for (const auto kind : {TopAbs_FACE,TopAbs_EDGE,TopAbs_VERTEX}) {
    NCollection_IndexedMap<TopoDS_Shape,TopTools_ShapeMapHasher> original,solids;
    TopExp::MapShapes(shape,kind,original);TopExp::MapShapes(result,kind,solids);
    if (original.Extent()!=solids.Extent()) throw std::runtime_error("IGES contains geometry outside the closed solids");
    for (int i=1;i<=original.Extent();++i)
      if (!solids.Contains(original(i))) throw std::runtime_error("IGES contains unrepresented geometry");
  }
  return result;
}
bool close(double a,double b) {return std::abs(a-b)<=std::max(1e-4,std::max(std::abs(a),std::abs(b))*1e-7);}
}
std::unique_ptr<Shape> read_iges(const std::string& path) noexcept {
  try {
    IGESControl_Reader reader;
    if (reader.ReadFile(path.c_str())!=IFSelect_RetDone)
      return failure("IGES_IMPORT_FAILED","Cannot read IGES file");
    const int roots=reader.NbRootsForTransfer();
    if (roots<=0 || reader.TransferRoots()!=roots)
      return failure("IGES_PARTIAL_IMPORT","Not all IGES geometry roots could be transferred");
    TopoDS_Shape result;
    try {result=closed_solids(reader.OneShape());}
    catch(const std::exception& error){return failure("IGES_NOT_SOLID",error.what());}
    auto checked_result=checked(result);
    if (!checked_result || !checked_result->ok()) return failure("IGES_IMPORT_FAILED","IGES BREP or solid measurements are invalid");
    return checked_result;
  } catch (const Standard_Failure& error) {return failure("IGES_IMPORT_FAILED",error.GetMessageString());}
  catch (...) {return failure("IGES_IMPORT_FAILED","Native IGES import failed");}
}
bool write_iges(const Shape& source,const std::string& path,std::string& error) noexcept {
  if (!source.ok()) {error="INVALID_SOURCE";return false;}
  try {
    BRepBuilderAPI_Copy copy(source.impl_->shape,true,false);
    const auto shape=closed_solids(copy.Shape());
    IGESControl_Writer writer("MM",1);
    if (!writer.AddShape(shape)) {error="IGES_EXPORT_FAILED";return false;}
    writer.ComputeModel();
    if (!writer.Write(path.c_str())) {error="IGES_EXPORT_FAILED";return false;}
    const auto restored=read_iges(path);
    if (!restored || !restored->ok() || restored->solid_count()!=source.solid_count()
        || !close(restored->volume_mm3(),source.volume_mm3())
        || !close(restored->surface_area_mm2(),source.surface_area_mm2())
        || !close(restored->extent_x_mm(),source.extent_x_mm())
        || !close(restored->extent_y_mm(),source.extent_y_mm())
        || !close(restored->extent_z_mm(),source.extent_z_mm())) {
      error="IGES_ROUND_TRIP_FAILED";return false;
    }
    error.clear();return true;
  } catch (...) {error="IGES_EXPORT_FAILED";return false;}
}
}
