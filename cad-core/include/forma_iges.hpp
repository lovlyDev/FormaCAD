#pragma once
#include "forma_core.hpp"
namespace forma {
std::unique_ptr<Shape> read_iges(const std::string&) noexcept;
bool write_iges(const Shape&, const std::string&, std::string&) noexcept;
}
