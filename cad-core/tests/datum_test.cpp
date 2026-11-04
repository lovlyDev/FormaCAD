#include "forma_datum.hpp"
#include "test_support.hpp"

#include <BRepAdaptor_Surface.hxx>
#include <BRepCheck_Analyzer.hxx>
#include <BRepGProp.hxx>
#include <BRep_Tool.hxx>
#include <GProp_GProps.hxx>
#include <STEPControl_Reader.hxx>
#include <STEPControl_Writer.hxx>
#include <TopExp_Explorer.hxx>
#include <TopoDS.hxx>
#include <chrono>
#include <cmath>
#include <filesystem>
#include <limits>

namespace {

constexpr forma::datum::Coordinates origin{2, -4, 7};

void near(double actual, double expected, const char* message) {
  require(std::abs(actual - expected) < 1e-6, message);
}

TopoDS_Shape of_type(const TopoDS_Shape& shape, TopAbs_ShapeEnum type) {
  TopExp_Explorer explorer(shape, type);
  require(explorer.More(), "STEP retained the datum topology type");
  return explorer.Current();
}

void verify_plane(const TopoDS_Shape& shape) {
  const auto face = TopoDS::Face(of_type(shape, TopAbs_FACE));
  const BRepAdaptor_Surface surface(face);
  require(surface.GetType() == GeomAbs_Plane, "Datum remains planar");
  near(surface.Plane().Distance(gp_Pnt(origin[0], origin[1], origin[2])), 0., "Datum plane origin");
  near(std::abs(surface.Plane().Axis().Direction().Y()), 1., "Datum plane normal");
  GProp_GProps properties;
  BRepGProp::SurfaceProperties(face, properties);
  near(properties.Mass(), 144., "Finite datum plane area");
}

void verify_axis(const TopoDS_Shape& shape) {
  GProp_GProps properties;
  BRepGProp::LinearProperties(of_type(shape, TopAbs_EDGE), properties);
  near(properties.Mass(), 16., "Datum axis length");
  near(properties.CentreOfMass().Distance(gp_Pnt(origin[0], origin[1], origin[2])),
       0., "Datum axis midpoint");
}

void verify_point(const TopoDS_Shape& shape) {
  const gp_Pnt point = BRep_Tool::Pnt(TopoDS::Vertex(of_type(shape, TopAbs_VERTEX)));
  near(point.Distance(gp_Pnt(origin[0], origin[1], origin[2])), 0., "Datum point coordinates");
}

void round_trip(const TopoDS_Shape& shape, void (*verify)(const TopoDS_Shape&)) {
  require(!shape.IsNull() && BRepCheck_Analyzer(shape).IsValid(), "Initial datum BREP");
  verify(shape);
  const auto path = std::filesystem::temp_directory_path() /
    ("forma-datum-" + std::to_string(std::chrono::steady_clock::now().time_since_epoch().count()) + ".step");
  STEPControl_Writer writer;
  require(writer.Transfer(shape, STEPControl_AsIs) == IFSelect_RetDone, "Transfer datum to STEP");
  require(writer.Write(path.string().c_str()) == IFSelect_RetDone, "Write datum STEP");
  STEPControl_Reader reader;
  require(reader.ReadFile(path.string().c_str()) == IFSelect_RetDone &&
          reader.TransferRoots() > 0, "Read datum STEP");
  const auto restored = reader.OneShape();
  require(!restored.IsNull() && BRepCheck_Analyzer(restored).IsValid(), "Restored datum BREP");
  verify(restored);
  std::filesystem::remove(path);
}

} // namespace

int main() {
  return run_test([] {
    using namespace forma::datum;
    round_trip(plane(origin, {0, 4, 0}, 12), verify_plane);
    round_trip(axis(origin, {1, 2, 3}, 16), verify_axis);
    round_trip(point(origin), verify_point);
    require(plane(origin, {0, 0, 0}, 12).IsNull(), "Reject zero plane normal");
    require(axis(origin, {0, 0, 1}, 0).IsNull(), "Reject zero axis span");
    require(point({0, 0, std::numeric_limits<double>::infinity()}).IsNull(), "Reject non-finite point");
  });
}
