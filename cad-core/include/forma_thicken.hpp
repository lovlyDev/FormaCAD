#pragma once
#include "forma_core.hpp"
namespace forma {
std::unique_ptr<Shape> thicken_face(const Shape&, const std::string& owner,
    const std::string& role, const std::string& occurrence, double offset_mm) noexcept;
}
