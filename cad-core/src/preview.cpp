#include "internal/shape_impl.hpp"
#include "internal/face_measurements.hpp"
#include "internal/edge_measurements.hpp"

#include <BRepMesh_IncrementalMesh.hxx>
#include <BRep_Tool.hxx>
#include <Poly_Triangulation.hxx>
#include <Standard_Failure.hxx>
#include <TopAbs_Orientation.hxx>
#include <TopAbs_ShapeEnum.hxx>
#include <TopExp_Explorer.hxx>
#include <TopoDS.hxx>
#include <TopoDS_Face.hxx>
#include <TopLoc_Location.hxx>

#include <algorithm>
#include <bit>
#include <cmath>
#include <cstdint>
#include <fstream>
#include <iomanip>
#include <limits>
#include <locale>
#include <sstream>
#include <stdexcept>
#include <vector>

namespace forma {
namespace {

constexpr std::size_t max_vertices = 2'000'000;
constexpr std::size_t max_bytes = 40 * 1024 * 1024;

void u32(std::vector<std::uint8_t>& out, std::uint32_t value) {
  out.push_back(static_cast<std::uint8_t>(value));
  out.push_back(static_cast<std::uint8_t>(value >> 8));
  out.push_back(static_cast<std::uint8_t>(value >> 16));
  out.push_back(static_cast<std::uint8_t>(value >> 24));
}

void f32(std::vector<std::uint8_t>& out, float value) {
  u32(out, std::bit_cast<std::uint32_t>(value));
}

void pad(std::vector<std::uint8_t>& bytes, std::uint8_t value) {
  while (bytes.size() % 4 != 0) bytes.push_back(value);
}

bool finite_point(const gp_Pnt& point) {
  return std::isfinite(point.X()) && std::isfinite(point.Y()) && std::isfinite(point.Z());
}

} // namespace

bool write_glb(const Shape& source, const std::string& path, std::string& code) noexcept {
  if (!source.ok()) {
    code = "INVALID_SOURCE";
    return false;
  }
  try {
    BRepMesh_IncrementalMesh mesher(source.impl_->shape, 0.1, false, 0.15, false);
    if (!mesher.IsDone()) {
      code = "TESSELLATION_FAILED";
      return false;
    }
    std::vector<float> positions;
    std::vector<std::uint32_t> indices;
    std::vector<std::size_t> face_triangles;
    std::vector<double> face_areas;
    std::vector<std::string> face_references;
    const auto edges = preview_edges(source.impl_->shape, source.impl_->topology);
    float minima[3] = {std::numeric_limits<float>::max(), std::numeric_limits<float>::max(),
                       std::numeric_limits<float>::max()};
    float maxima[3] = {std::numeric_limits<float>::lowest(), std::numeric_limits<float>::lowest(),
                       std::numeric_limits<float>::lowest()};
    for (TopExp_Explorer it(source.impl_->shape, TopAbs_FACE); it.More(); it.Next()) {
      const TopoDS_Face face = TopoDS::Face(it.Current());
      face_triangles.push_back(0);
      face_areas.push_back(face_area_mm2(face));
      face_references.push_back(topology_reference_json(source.impl_->topology, face));
      TopLoc_Location location;
      const auto& triangulation = BRep_Tool::Triangulation(face, location);
      if (triangulation.IsNull() || triangulation->NbNodes() == 0) continue;
      const std::size_t base = positions.size() / 3;
      if (base + static_cast<std::size_t>(triangulation->NbNodes()) > max_vertices) {
        code = "TESSELLATION_TOO_LARGE";
        return false;
      }
      for (int node = 1; node <= triangulation->NbNodes(); ++node) {
        const gp_Pnt point = triangulation->Node(node).Transformed(location.Transformation());
        if (!finite_point(point)) {
          code = "TESSELLATION_INVALID";
          return false;
        }
        const float coordinates[3] = {static_cast<float>(point.X() / 1000.0),
                                      static_cast<float>(point.Z() / 1000.0),
                                      static_cast<float>(-point.Y() / 1000.0)};
        for (int axis = 0; axis < 3; ++axis) {
          positions.push_back(coordinates[axis]);
          minima[axis] = std::min(minima[axis], coordinates[axis]);
          maxima[axis] = std::max(maxima[axis], coordinates[axis]);
        }
      }
      for (int triangle = 1; triangle <= triangulation->NbTriangles(); ++triangle) {
        int a, b, c;
        triangulation->Triangle(triangle).Get(a, b, c);
        if (a < 1 || b < 1 || c < 1 || a > triangulation->NbNodes() ||
            b > triangulation->NbNodes() || c > triangulation->NbNodes()) {
          code = "TESSELLATION_INVALID";
          return false;
        }
        if (face.Orientation() == TopAbs_REVERSED) std::swap(b, c);
        indices.push_back(static_cast<std::uint32_t>(base + a - 1));
        indices.push_back(static_cast<std::uint32_t>(base + b - 1));
        indices.push_back(static_cast<std::uint32_t>(base + c - 1));
        ++face_triangles.back();
      }
      if (positions.size() * sizeof(float) + indices.size() * sizeof(std::uint32_t) > max_bytes) {
        code = "TESSELLATION_TOO_LARGE";
        return false;
      }
    }
    if (positions.empty() || indices.empty()) {
      code = "TESSELLATION_EMPTY";
      return false;
    }
    std::vector<std::uint8_t> binary;
    binary.reserve(positions.size() * 4 + indices.size() * 4);
    for (float value : positions) f32(binary, value);
    const std::size_t index_offset = binary.size();
    for (std::uint32_t value : indices) u32(binary, value);
    pad(binary, 0);
    std::ostringstream json;
    json.imbue(std::locale::classic());
    json << std::setprecision(9);
    json << "{\"asset\":{\"version\":\"2.0\",\"generator\":\"Forma CAD\"},"
            "\"scene\":0,\"scenes\":[{\"nodes\":[0]}],\"nodes\":[{\"mesh\":0,\"name\":\"Body\"}],"
            "\"meshes\":[{\"primitives\":[{\"attributes\":{\"POSITION\":0},\"indices\":1,"
            "\"extras\":{\"formaFaceTriangleCounts\":[";
    for (std::size_t face = 0; face < face_triangles.size(); ++face) {
      if (face != 0) json << ',';
      json << face_triangles[face];
    }
    json << "],\"formaFaceAreasMm2\":[";
    json << std::setprecision(std::numeric_limits<double>::max_digits10);
    for (std::size_t face = 0; face < face_areas.size(); ++face) {
      if (face != 0) json << ',';
      json << face_areas[face];
    }
    json << "],\"formaFaceReferences\":[";
    for (std::size_t face = 0; face < face_references.size(); ++face) {
      if (face != 0) json << ',';
      json << face_references[face];
    }
    json << "],\"formaEdges\":[";
    for (std::size_t edge = 0; edge < edges.size(); ++edge) {
      if (edge != 0) json << ',';
      json << "{\"lengthMm\":" << edges[edge].length_mm << ",\"semanticKey\":";
      if (edges[edge].semantic_key.empty()) json << "null";
      else json << '"' << edges[edge].semantic_key << '"';
      json << ",\"topologyRef\":" << edges[edge].topology_ref;
      json << ",\"radiusMm\":";
      if (edges[edge].radius_mm) json << *edges[edge].radius_mm;
      else json << "null";
      json << ",\"points\":[";
      for (std::size_t point = 0; point < edges[edge].points_m.size(); ++point) {
        if (point != 0) json << ',';
        const auto& coordinates = edges[edge].points_m[point];
        json << '[' << coordinates[0] << ',' << coordinates[1] << ',' << coordinates[2] << ']';
      }
      json << "]}";
    }
    json << "]}}]}],"
            "\"buffers\":[{\"byteLength\":" << binary.size() << "}],"
            "\"bufferViews\":[{\"buffer\":0,\"byteOffset\":0,\"byteLength\":"
         << index_offset << ",\"target\":34962},{\"buffer\":0,\"byteOffset\":"
         << index_offset << ",\"byteLength\":" << binary.size() - index_offset
         << ",\"target\":34963}],\"accessors\":[{\"bufferView\":0,\"componentType\":5126,"
            "\"count\":" << positions.size() / 3
         << ",\"type\":\"VEC3\",\"min\":[" << minima[0] << ',' << minima[1] << ','
         << minima[2] << "],\"max\":[" << maxima[0] << ',' << maxima[1] << ',' << maxima[2]
         << "]},{\"bufferView\":1,\"componentType\":5125,\"count\":" << indices.size()
         << ",\"type\":\"SCALAR\"}]}";
    std::string json_text = json.str();
    while (json_text.size() % 4 != 0) json_text.push_back(' ');
    const std::size_t total = 28 + json_text.size() + binary.size();
    if (total > max_bytes || total > std::numeric_limits<std::uint32_t>::max()) {
      code = "TESSELLATION_TOO_LARGE";
      return false;
    }
    std::vector<std::uint8_t> glb;
    glb.reserve(total);
    u32(glb, 0x46546C67); // glTF
    u32(glb, 2);
    u32(glb, static_cast<std::uint32_t>(total));
    u32(glb, static_cast<std::uint32_t>(json_text.size()));
    u32(glb, 0x4E4F534A); // JSON
    glb.insert(glb.end(), json_text.begin(), json_text.end());
    u32(glb, static_cast<std::uint32_t>(binary.size()));
    u32(glb, 0x004E4942); // BIN
    glb.insert(glb.end(), binary.begin(), binary.end());
    std::ofstream output(path, std::ios::binary);
    if (!output || !output.write(reinterpret_cast<const char*>(glb.data()),
                                 static_cast<std::streamsize>(glb.size()))) {
      code = "PREVIEW_WRITE_FAILED";
      return false;
    }
    code.clear();
    return true;
  } catch (const Standard_Failure&) {
    code = "OCCT_ERROR";
  } catch (const std::runtime_error&) {
    code = "INVALID_FACE_AREA";
  } catch (...) {
    code = "PREVIEW_FAILED";
  }
  return false;
}

} // namespace forma
