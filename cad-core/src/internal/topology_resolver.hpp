#pragma once
#include "topology_catalog.hpp"
#include <TopAbs_ShapeEnum.hxx>

namespace forma {
struct ResolvedTopology {
  TopoDS_Shape entity;
  std::string code;
  std::string detail;
  bool valid() const { return code.empty() && !entity.IsNull(); }
};
// Internal OCCT result only. Public FFI callers catch every exception.
ResolvedTopology resolve_topology(const TopoDS_Shape& body,
  const TopologyCatalog& catalogue, TopAbs_ShapeEnum kind,
  const std::string& owner, const std::string& role, const std::string& path);
}
