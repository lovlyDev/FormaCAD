#pragma once

#include "forma_core.hpp"
#include <TopoDS_Shape.hxx>
#include "topology_catalog.hpp"

namespace forma {

struct Shape::Impl {
  TopoDS_Shape shape;
  TopologyCatalog topology;
  double volume = 0.0;
  double area = 0.0;
  int faces = 0;
  int edges = 0;
  double extents[3] = {0.0, 0.0, 0.0};
  std::string code;
  std::string message;
};

} // namespace forma
