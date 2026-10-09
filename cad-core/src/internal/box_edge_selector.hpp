#pragma once

#include <TopoDS_Shape.hxx>

#include <string>
#include <vector>

namespace forma {

// Unique, full-span linear edges on two extreme sides of a shape. Empty keys
// mean the edge cannot be identified unambiguously after regeneration.
std::vector<std::string> box_edge_keys(const TopoDS_Shape& shape);

} // namespace forma
