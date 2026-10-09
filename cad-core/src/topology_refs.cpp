#include "internal/shape_impl.hpp"
#include "internal/topology_resolver.hpp"
#include <BRepPrimAPI_MakeBox.hxx>
#include <BRepFilletAPI_MakeFillet.hxx>
#include <NCollection_IndexedMap.hxx>
#include <TopTools_ShapeMapHasher.hxx>
#include <TopExp.hxx>
#include <TopExp_Explorer.hxx>
#include <TopoDS.hxx>
#include <TopoDS_Face.hxx>
#include <Standard_Failure.hxx>
#include <gp_Pnt.hxx>
#include <algorithm>
#include <array>
#include <cmath>
#include <sstream>

namespace forma {
namespace {
bool valid_id(const std::string& id) {
  return !id.empty() && id.size() <= 80 && std::all_of(id.begin(), id.end(), [](unsigned char c) {
    return (c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z') ||
           (c >= '0' && c <= '9') || c == '-' || c == '_';
  });
}
bool member(const TopoDS_Shape& shape, const TopoDS_Shape& entity) {
  for (TopExp_Explorer it(shape, entity.ShapeType()); it.More(); it.Next())
    if (it.Current().IsSame(entity)) return true;
  return false;
}
} // namespace

TopologyCatalog transformed_catalog(const TopologyCatalog& catalog,
    BRepBuilderAPI_Transform& builder, const TopoDS_Shape& output) {
  TopologyCatalog result;
  for (const auto& entry : catalog) {
    const auto moved = builder.ModifiedShape(entry.entity);
    if (moved.IsNull() || moved.ShapeType() != entry.entity.ShapeType() || !member(output, moved))
      return {}; // Atomic catalog: any lost provenance invalidates the bounded route.
    auto copy = entry;
    copy.entity = moved;
    result.push_back(std::move(copy));
  }
  return result;
}

std::string topology_reference_json(const TopologyCatalog& catalog, const TopoDS_Shape& entity) {
  const TopologyEntry* found = nullptr;
  for (const auto& entry : catalog) {
    if (!entry.entity.IsSame(entity)) continue;
    if (found) return "null";
    found = &entry;
  }
  if (!found) return "null";
  std::ostringstream json;
  json << "{\"schemaVersion\":1,\"kind\":\"" << (entity.ShapeType() == TopAbs_EDGE ? "edge" : "face")
       << "\",\"ownerFeatureId\":\"" << found->owner << "\",\"role\":\"" << found->role
       << "\",\"occurrencePath\":[";
  for (std::size_t i = 0; i < found->path.size(); ++i) {
    if (i) json << ',';
    json << '"' << found->path[i] << '"';
  }
  return json.str() + "]}";
}

std::unique_ptr<Shape> make_referenced_box(double width, double depth, double height,
    const std::string& owner) noexcept {
  if (!valid_id(owner)) return failure("INVALID_TOPOLOGY_REFERENCE", "Invalid owner ID");
  if (!std::isfinite(width) || !std::isfinite(depth) || !std::isfinite(height) ||
      width <= 0 || depth <= 0 || height <= 0 || width > 10000 || depth > 10000 || height > 10000)
    return failure("INVALID_DIMENSION", "Positive bounded rectangular extrusion required");
  try {
    BRepPrimAPI_MakeBox builder(gp_Pnt(-width / 2, -depth / 2, 0), width, depth, height);
    auto output = checked(builder.Shape());
    if (!output->ok()) return output;
    // Named constructor faces, not extrema guessed from an arbitrary existing solid.
    const std::array<TopoDS_Face, 6> faces = {builder.BackFace(), builder.FrontFace(),
      builder.LeftFace(), builder.RightFace(), builder.BottomFace(), builder.TopFace()};
    const std::array<std::string, 6> names = {"xmin", "xmax", "ymin", "ymax", "zmin", "zmax"};
    for (std::size_t i = 0; i < faces.size(); ++i)
      output->impl_->topology.push_back({faces[i], owner, "box-face:" + names[i], {}});
    NCollection_IndexedMap<TopoDS_Shape, TopTools_ShapeMapHasher> edges;
    TopExp::MapShapes(output->impl_->shape, TopAbs_EDGE, edges);
    for (int i = 1; i <= edges.Extent(); ++i) {
      std::vector<std::string> adjacent;
      for (std::size_t f = 0; f < faces.size(); ++f)
        if (member(faces[f], edges(i))) adjacent.push_back(names[f]);
      if (adjacent.size() != 2 || adjacent[0][0] == adjacent[1][0])
        return failure("TOPOLOGY_REFERENCE_AMBIGUOUS", "Rectangle constructor adjacency is ambiguous");
      const char axis = adjacent[0][0] != 'x' && adjacent[1][0] != 'x' ? 'x' :
                        adjacent[0][0] != 'y' && adjacent[1][0] != 'y' ? 'y' : 'z';
      output->impl_->topology.push_back({edges(i), owner,
        std::string("box-edge:") + axis + ':' + adjacent[0] + ':' + adjacent[1], {}});
    }
    return output;
  } catch (const Standard_Failure& error) { return failure("OCCT_ERROR", error.GetMessageString()); }
  catch (...) { return failure("NATIVE_ERROR", "Referenced rectangle construction failed"); }
}

std::unique_ptr<Shape> topology_occurrence(const Shape& source, const std::string& id) noexcept {
  if (!source.ok()) return failure("INVALID_SOURCE", "Invalid topology source");
  if (!valid_id(id)) return failure("INVALID_TOPOLOGY_REFERENCE", "Invalid occurrence ID");
  try {
    auto output = checked(source.impl_->shape);
    if (!output->ok()) return output;
    output->impl_->topology = source.impl_->topology;
    for (auto& entry : output->impl_->topology) {
      if (entry.path.size() >= 64) {
        output->impl_->topology.clear();
        return output; // Preserve valid geometry beyond bounded provenance support.
      }
      if (entry.owner == id ||
          std::find(entry.path.begin(), entry.path.end(), id) != entry.path.end())
        return failure("INVALID_TOPOLOGY_REFERENCE", "Occurrence path exceeds supported bounds");
      entry.path.push_back(id);
    }
    return output;
  } catch (...) { return failure("NATIVE_ERROR", "Topology occurrence failed"); }
}

std::unique_ptr<Shape> fillet_referenced_edge(const Shape& source, const std::string& owner,
    const std::string& role, const std::string& path, double radius) noexcept {
  if (!source.ok()) return failure("INVALID_SOURCE", "Invalid fillet source");
  if (!std::isfinite(radius) || radius <= 0 || radius > 10000)
    return failure("INVALID_DIMENSION", "Invalid fillet radius");
  try {
    const auto selected = resolve_topology(source.impl_->shape, source.impl_->topology, TopAbs_EDGE, owner, role, path);
    if (!selected.valid()) return failure(selected.code.c_str(), selected.detail.c_str());
    BRepFilletAPI_MakeFillet builder(source.impl_->shape);
    builder.Add(radius, TopoDS::Edge(selected.entity));
    builder.Build();
    if (!builder.IsDone()) return failure("FILLET_FAILED", "Selected edge cannot be filleted");
    return checked(builder.Shape()); // Topological modifiers deliberately invalidate v1 catalog.
  } catch (const Standard_Failure& error) { return failure("FILLET_FAILED", error.GetMessageString()); }
  catch (...) { return failure("NATIVE_ERROR", "Referenced fillet failed"); }
}
} // namespace forma
