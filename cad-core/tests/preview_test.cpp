#include "forma_core.hpp"
#include "test_support.hpp"

#include <cstdint>
#include <filesystem>
#include <fstream>
#include <iterator>
#include <string>
#include <vector>

static std::uint32_t u32(const std::vector<unsigned char>& bytes, std::size_t offset) {
  return std::uint32_t(bytes.at(offset)) | (std::uint32_t(bytes.at(offset + 1)) << 8) |
         (std::uint32_t(bytes.at(offset + 2)) << 16) |
         (std::uint32_t(bytes.at(offset + 3)) << 24);
}

int main() {
  return run_test([] {
    auto shape = forma::make_box(60.0, 40.0, 10.0);
    require(shape && shape->ok(), "preview test shape failed");
    const auto path = std::filesystem::temp_directory_path() / "forma-cad-core-preview.glb";
    std::string error;
    require(forma::write_glb(*shape, path.string(), error), "GLB export failed");
    std::ifstream input(path, std::ios::binary);
    const std::vector<unsigned char> bytes(std::istreambuf_iterator<char>{input}, {});
    input.close();
    require(bytes.size() > 100, "preview is empty");
    require(u32(bytes, 0) == 0x46546C67 && u32(bytes, 4) == 2,
            "preview is not glTF binary version 2");
    require(u32(bytes, 8) == bytes.size(), "GLB declared size is wrong");
    require(u32(bytes, 16) == 0x4E4F534A, "GLB JSON chunk is missing");
    const std::size_t json_length = u32(bytes, 12);
    const std::string json(bytes.begin() + 20, bytes.begin() + 20 + json_length);
    require(json.find("\"POSITION\"") != std::string::npos, "preview positions are missing");
    require(json.find("\"formaFaceTriangleCounts\"") != std::string::npos,
            "preview face mapping is missing");
    require(json.find("\"formaFaceAreasMm2\"") != std::string::npos,
            "preview exact face areas are missing");
    require(json.find("\"formaEdges\"") != std::string::npos,
            "preview exact edge lengths are missing");
    require(json.find("\"radiusMm\"") != std::string::npos,
            "preview exact circular edge radii are missing");
    require(json.find("\"semanticKey\":\"box-edge:x:ymin:zmin\"") != std::string::npos,
            "preview semantic edge key is missing");
    require(u32(bytes, 20 + json_length + 4) == 0x004E4942, "GLB binary chunk is missing");
    std::filesystem::remove(path);
  });
}
