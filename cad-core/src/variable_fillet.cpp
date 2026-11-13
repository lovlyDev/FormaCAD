#include "forma_variable_fillet.hpp"
#include "internal/shape_impl.hpp"
#include "internal/topology_resolver.hpp"
#include <BRepBuilderAPI_Copy.hxx>
#include <BRepFilletAPI_MakeFillet.hxx>
#include <BRep_Tool.hxx>
#include <Precision.hxx>
#include <Standard_Failure.hxx>
#include <TopExp.hxx>
#include <TopoDS.hxx>
#include <TopoDS_Edge.hxx>
#include <TopoDS_Vertex.hxx>
#include <cmath>
#include <stdexcept>

namespace forma {
namespace {
struct Input {
  TopoDS_Shape body;
  TopoDS_Edge edge;
  TopoDS_Vertex first, last;
  std::string code, detail;
};
Input prepare(const TopoDS_Shape& shape, const TopologyCatalog& catalog,
    const std::string& owner, const std::string& role, const std::string& path) {
  Input result;
  const auto resolved = resolve_topology(shape, catalog, TopAbs_EDGE, owner, role, path);
  if (!resolved.valid()) { result.code=resolved.code; result.detail=resolved.detail; return result; }
  // Fillet may update p-curves/tolerances: source BREP must remain independent.
  BRepBuilderAPI_Copy copy(shape, true, false);
  result.body=copy.Shape();
  result.edge=TopoDS::Edge(copy.ModifiedShape(resolved.entity));
  TopExp::Vertices(result.edge, result.first, result.last, false);
  if (result.first.IsNull() || result.last.IsNull() || result.first.IsSame(result.last)
      || BRep_Tool::Pnt(result.first).Distance(BRep_Tool::Pnt(result.last)) <= Precision::Confusion()) {
    result.code="VARIABLE_FILLET_CLOSED_EDGE";
    result.detail="Variable fillet requires two distinct edge endpoints";
    return result;
  }
  BRepFilletAPI_MakeFillet contour(result.body);
  contour.Add(result.edge);
  const int index=contour.Contour(result.edge);
  if (index==0 || contour.NbEdges(index)!=1) {
    result.code="VARIABLE_FILLET_CHAIN_UNSUPPORTED";
    result.detail="Select one open edge without automatic tangent-chain expansion";
  }
  return result;
}
}
std::unique_ptr<EdgeEndpoints> variable_fillet_endpoints(const Shape& source,
    const std::string& owner, const std::string& role, const std::string& path) noexcept {
  try {
    auto result=std::make_unique<EdgeEndpoints>();
    if (!source.ok()) { result->code_="INVALID_SOURCE"; result->detail_="Invalid fillet source"; return result; }
    const auto input=prepare(source.impl_->shape,source.impl_->topology,owner,role,path);
    if (!input.code.empty()) { result->code_=input.code; result->detail_=input.detail; return result; }
    const auto a=BRep_Tool::Pnt(input.first), b=BRep_Tool::Pnt(input.last);
    result->start_={a.X(),a.Y(),a.Z()}; result->end_={b.X(),b.Y(),b.Z()};
    return result;
  } catch (...) { return nullptr; }
}
std::unique_ptr<Shape> variable_fillet(const Shape& source, const std::string& owner,
    const std::string& role, const std::string& path, double start, double end) noexcept {
  if (!source.ok()) return failure("INVALID_SOURCE","Invalid variable fillet source");
  if (!std::isfinite(start) || !std::isfinite(end) || start<=Precision::Confusion()
      || end<=Precision::Confusion() || start>10000. || end>10000.)
    return failure("INVALID_DIMENSION","Both radii must be finite and positive");
  try {
    const auto input=prepare(source.impl_->shape,source.impl_->topology,owner,role,path);
    if (!input.code.empty()) return failure(input.code.c_str(),input.detail.c_str());
    BRepFilletAPI_MakeFillet operation(input.body);
    operation.Add(input.edge);
    const int index=operation.Contour(input.edge);
    const auto first=operation.FirstVertex(index);
    if (first.IsSame(input.first)) operation.SetRadius(start,end,index,1);
    else if (first.IsSame(input.last)) operation.SetRadius(end,start,index,1);
    else return failure("VARIABLE_FILLET_FAILED","Cannot bind radius law to the selected endpoints");
    operation.Build();
    if (!operation.IsDone()) return failure("VARIABLE_FILLET_FAILED","Radii cannot be applied to this edge");
    auto result=checked(operation.Shape());
    if (!result || !result->ok()) return failure("VARIABLE_FILLET_FAILED","Variable fillet produced an invalid BREP");
    return result;
  } catch (const Standard_Failure& error) {
    return failure("VARIABLE_FILLET_FAILED",error.GetMessageString());
  } catch (...) { return failure("VARIABLE_FILLET_FAILED","Native variable fillet failed"); }
}
}
