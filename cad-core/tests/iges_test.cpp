#include "forma_iges.hpp"
#include "forma_variable_fillet.hpp"
#include "test_support.hpp"
#include <BRepBuilderAPI_MakeFace.hxx>
#include <BRepPrimAPI_MakeBox.hxx>
#include <IGESControl_Writer.hxx>
#include <gp_Pln.hxx>
#include <gp.hxx>
#include <chrono>
#include <cmath>
#include <filesystem>
#include <fstream>
#include <iomanip>
#include <iostream>

int main() {return run_test([] {
  const auto folder=std::filesystem::temp_directory_path()/
      ("forma-iges-"+std::to_string(std::chrono::steady_clock::now().time_since_epoch().count()));
  require(std::filesystem::create_directory(folder),"Own IGES test folder");
  const auto box=forma::make_box(40.,20.,10.);
  const auto cylinder=forma::make_cylinder(3.,10.);
  const auto placed=forma::translate(*cylinder,60.,0.,0.);
  const auto compound=forma::make_compound(*box,*placed);
  const auto torus=forma::make_torus(12.,3.);
  const auto referenced=forma::make_referenced_box(40.,20.,10.,"pad");
  const auto variable=forma::variable_fillet(*referenced,"pad","box-edge:z:xmax:ymin","",1.25,3.5);
  require(variable->ok(),"Variable fillet exchange fixture");
  for (const auto* source : {box.get(),compound.get(),torus.get(),variable.get()}) {
    std::string error;const auto path=folder/"round-trip.igs";
    const bool exported=forma::write_iges(*source,path.string(),error);
    require(exported,error.c_str());
    const auto result=forma::read_iges(path.string());
    require(result && result->ok(),result ? result->error_message().c_str() : "Null IGES result");
    require(result->solid_count()==source->solid_count(),"Every solid is preserved");
    require(std::abs(result->volume_mm3()-source->volume_mm3())<1e-3,"IGES volume round-trip");
    require(std::abs(result->surface_area_mm2()-source->surface_area_mm2())<1e-3,"IGES area round-trip");
    std::cout<<std::setprecision(15)<<"IGES solids "<<result->solid_count()<<" V "<<result->volume_mm3()<<" A "<<result->surface_area_mm2()<<"\n";
    const auto step=folder/"canonical.step";
    require(forma::write_step(*result,step.string(),error),"Canonical STEP export");
    const auto restored=forma::read_step(step.string());
    require(restored->ok(),"Canonical STEP BREP");
    require(std::abs(restored->volume_mm3()-result->volume_mm3())<1e-3,"IGES → STEP volume");
    require(std::abs(restored->surface_area_mm2()-result->surface_area_mm2())<1e-3,"IGES → STEP area");
  }
  // Independent writer verifies import respects IGES inch units.
  IGESControl_Writer inches("INCH",1);
  require(inches.AddShape(BRepPrimAPI_MakeBox(40.,20.,10.).Shape()),"Independent inch source");
  require(inches.Write((folder/"inches.iges").string().c_str()),"Independent inch file");
  const auto inch_result=forma::read_iges((folder/"inches.iges").string());
  require(inch_result->ok(),"Inch solid import");
  require(std::abs(inch_result->volume_mm3()-8000.)<1e-3,"Inch header normalized into mm");
  IGESControl_Writer surface("MM",0);
  require(surface.AddShape(BRepBuilderAPI_MakeFace(gp_Pln(gp::XOY()),0.,40.,0.,20.).Shape()),"Surface fixture");
  require(surface.Write((folder/"surface.igs").string().c_str()),"Surface IGES file");
  const auto rejected=forma::read_iges((folder/"surface.igs").string());
  require(!rejected->ok() && rejected->error_code()=="IGES_NOT_SOLID","No guessed solid for open surfaces");
  {std::ofstream junk(folder/"invalid.igs");junk<<"invalid IGES";}
  require(!forma::read_iges((folder/"invalid.igs").string())->ok(),"Malformed IGES rejected");
  require(!forma::read_iges((folder/"missing.igs").string())->ok(),"Missing input rejected");
  for (const auto& file:std::filesystem::directory_iterator(folder))std::filesystem::remove(file.path());
  std::filesystem::remove(folder);
});}
