#include "forma_core.hpp"
#include "internal/profile_face.hpp"
#include "internal/transform_axis.hpp"
#include <BRepPrimAPI_MakeRevol.hxx>
#include <Standard_Failure.hxx>
#include <gp_Ax1.hxx>
#include <gp_Dir.hxx>
#include <gp_Pnt.hxx>
#include <gp_Vec.hxx>
#include <algorithm>
#include <numbers>
#include <stdexcept>

namespace forma {
std::unique_ptr<Shape> make_profile_revolve(const double* coordinates, std::size_t coordinate_count,
    const std::size_t* offsets, std::size_t offset_count, int plane, double ox, double oy, double oz,
    double ax, double ay, double az, double dx, double dy, double dz, double angle) noexcept {
  if (!valid_transform_axis(ax,ay,az,dx,dy,dz) || !std::isfinite(angle) || angle==0 || std::abs(angle)>360)
    return failure("INVALID_REVOLUTION", "Revolution axis and nonzero angle must be finite and bounded");
  try {
    const auto face = profile_face(coordinates,coordinate_count,offsets,offset_count,plane,ox,oy,oz);
    const gp_Vec normal = plane==0 ? gp_Vec(0,0,1) : plane==1 ? gp_Vec(0,-1,0) : gp_Vec(1,0,0);
    const gp_Dir axis(dx,dy,dz);
    if (std::abs(normal.Dot(gp_Vec(axis)))>1e-9 || std::abs(normal.Dot(gp_Vec(ax-ox,ay-oy,az-oz)))>1e-7)
      return failure("INVALID_REVOLUTION", "Revolution axis must lie in the profile plane");
    const gp_Vec radial = gp_Vec(axis).Crossed(normal);
    double minimum=0,maximum=0;
    for (std::size_t i=0;i<coordinate_count;i+=2) {
      const gp_Vec position = plane==0 ? gp_Vec(ox+coordinates[i]-ax,oy+coordinates[i+1]-ay,oz-az) :
          plane==1 ? gp_Vec(ox+coordinates[i]-ax,oy-ay,oz+coordinates[i+1]-az) :
                     gp_Vec(ox-ax,oy+coordinates[i]-ay,oz+coordinates[i+1]-az);
      const double radius=position.Dot(radial);
      minimum=std::min(minimum,radius); maximum=std::max(maximum,radius);
    }
    if (minimum < -1e-7 && maximum > 1e-7)
      return failure("INVALID_REVOLUTION", "Revolution profile must not cross the axis");
    BRepPrimAPI_MakeRevol builder(face,gp_Ax1(gp_Pnt(ax,ay,az),axis),angle*std::numbers::pi/180.0,true);
    if (!builder.IsDone()) return failure("INVALID_REVOLUTION", "Revolution did not produce a closed solid");
    return checked(builder.Shape());
  } catch (const std::invalid_argument& error) { return failure("INVALID_SKETCH", error.what()); }
    catch (const Standard_Failure& error) { return failure("OCCT_ERROR", error.GetMessageString()); }
    catch (...) { return failure("NATIVE_ERROR", "Profile revolution failed"); }
}
} // namespace forma
