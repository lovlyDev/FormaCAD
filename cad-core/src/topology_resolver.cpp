#include "internal/topology_resolver.hpp"
#include <TopExp_Explorer.hxx>
#include <algorithm>
#include <array>
#include <set>

namespace forma {
namespace {
bool identifier(const std::string& value) {
  return !value.empty() && value.size() <= 80 &&
    std::all_of(value.begin(), value.end(), [](unsigned char c) {
      return (c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z') ||
        (c >= '0' && c <= '9') || c == '_' || c == '-';
    });
}
bool valid_path(const std::string& owner, const std::string& path) {
  if (path.size() > 64 * 80 + 63) return false;
  if (path.empty()) return true;
  std::set<std::string> seen;
  std::size_t start = 0;
  while (start <= path.size()) {
    const auto end = path.find('/', start);
    const auto part = path.substr(start, end == std::string::npos ? end : end - start);
    if (!identifier(part) || part == owner || !seen.insert(part).second || seen.size() > 64)
      return false;
    if (end == std::string::npos) break;
    start = end + 1;
  }
  return true;
}
bool valid_role(TopAbs_ShapeEnum kind, const std::string& role) {
  static const std::array<std::string, 14> edges = {
    "box-edge:x:ymin:zmin", "box-edge:x:ymin:zmax", "box-edge:x:ymax:zmin", "box-edge:x:ymax:zmax",
    "box-edge:y:xmin:zmin", "box-edge:y:xmin:zmax", "box-edge:y:xmax:zmin", "box-edge:y:xmax:zmax",
    "box-edge:z:xmin:ymin", "box-edge:z:xmin:ymax", "box-edge:z:xmax:ymin", "box-edge:z:xmax:ymax",
    "cylinder-edge:bottom", "cylinder-edge:top"};
  static const std::array<std::string, 9> faces = {
    "box-face:xmin", "box-face:xmax", "box-face:ymin", "box-face:ymax", "box-face:zmin", "box-face:zmax",
    "cylinder-face:bottom", "cylinder-face:top", "cylinder-face:side"};
  if (kind == TopAbs_EDGE) return std::find(edges.begin(), edges.end(), role) != edges.end();
  if (kind == TopAbs_FACE) return std::find(faces.begin(), faces.end(), role) != faces.end();
  return false;
}
std::string encoded(const std::vector<std::string>& path) {
  std::string result;
  for (const auto& id : path) { if (!result.empty()) result += '/'; result += id; }
  return result;
}
ResolvedTopology error(const char* code, const char* detail) { return {{}, code, detail}; }
}
ResolvedTopology resolve_topology(const TopoDS_Shape& body, const TopologyCatalog& catalogue,
    TopAbs_ShapeEnum kind, const std::string& owner, const std::string& role, const std::string& path) {
  if (!identifier(owner) || !valid_role(kind, role) || !valid_path(owner, path))
    return error("INVALID_TOPOLOGY_REFERENCE", "Invalid bounded reference kind, owner, role or occurrence path");
  if (catalogue.empty())
    return error("TOPOLOGY_REFERENCE_UNSUPPORTED", "This operation chain has no supported provenance");
  const TopologyEntry* selected = nullptr;
  for (const auto& entry : catalogue) {
    if (entry.entity.ShapeType() != kind || entry.owner != owner || entry.role != role || encoded(entry.path) != path)
      continue;
    if (selected) return error("TOPOLOGY_REFERENCE_AMBIGUOUS", "Reference resolves to multiple entities");
    selected = &entry;
  }
  if (selected) {
    for (TopExp_Explorer it(body, kind); it.More(); it.Next())
      // ModifiedShape preserves geometric identity but can return a different
      // orientation. Use the actual occurrence in the body's BREP shell.
      if (it.Current().IsSame(selected->entity)) return {it.Current(), {}, {}};
  }
  return error("TOPOLOGY_REFERENCE_UNRESOLVED", "Reference owner or occurrence is absent from the selected body");
}
}
