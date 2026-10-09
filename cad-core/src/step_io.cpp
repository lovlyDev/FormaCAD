#include "internal/shape_impl.hpp"

#include <IFSelect_ReturnStatus.hxx>
#include <STEPControl_Reader.hxx>
#include <STEPControl_Writer.hxx>
#include <Standard_Failure.hxx>

namespace forma {

std::unique_ptr<Shape> read_step(const std::string& path) noexcept {
  try {
    STEPControl_Reader reader;
    if (reader.ReadFile(path.c_str()) != IFSelect_RetDone || reader.TransferRoots() <= 0)
      return failure("STEP_IMPORT_FAILED", "Could not read STEP geometry");
    return checked(reader.OneShape());
  } catch (const Standard_Failure& error) {
    return failure("OCCT_ERROR", error.GetMessageString());
  } catch (...) {
    return failure("NATIVE_ERROR", "STEP import failed");
  }
}

bool write_step(const Shape& source, const std::string& path, std::string& code) noexcept {
  if (!source.ok()) {
    code = "INVALID_SOURCE";
    return false;
  }
  try {
    STEPControl_Writer writer;
    if (writer.Transfer(source.impl_->shape, STEPControl_AsIs) != IFSelect_RetDone ||
        writer.Write(path.c_str()) != IFSelect_RetDone) {
      code = "STEP_EXPORT_FAILED";
      return false;
    }
    code.clear();
    return true;
  } catch (const Standard_Failure&) {
    code = "OCCT_ERROR";
  } catch (...) {
    code = "NATIVE_ERROR";
  }
  return false;
}

} // namespace forma
