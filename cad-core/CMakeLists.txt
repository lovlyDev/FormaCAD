cmake_minimum_required(VERSION 3.24)
project(FormaCadCore VERSION 2.0.0 LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)

if(DEFINED FORMA_OCCT_ROOT)
  foreach(candidate IN ITEMS "${FORMA_OCCT_ROOT}/include/opencascade" "${FORMA_OCCT_ROOT}/inc")
    if(EXISTS "${candidate}/TopoDS_Shape.hxx")
      set(OpenCASCADE_INCLUDE_DIR "${candidate}")
      break()
    endif()
  endforeach()
  foreach(candidate IN ITEMS "${FORMA_OCCT_ROOT}/lib" "${FORMA_OCCT_ROOT}/lib64" "${FORMA_OCCT_ROOT}/win64/vc14/lib")
    if(EXISTS "${candidate}/TKernel.lib" OR EXISTS "${candidate}/libTKernel.a" OR EXISTS "${candidate}/libTKernel.so" OR EXISTS "${candidate}/libTKernel.dylib")
      set(OpenCASCADE_LIBRARY_DIR "${candidate}")
      break()
    endif()
  endforeach()
  set(OpenCASCADE_BINARY_DIR "${OpenCASCADE_BINARY_DIR}")
  if(NOT DEFINED OpenCASCADE_INCLUDE_DIR OR NOT DEFINED OpenCASCADE_LIBRARY_DIR)
    message(FATAL_ERROR "FORMA_OCCT_ROOT does not contain OCCT headers and libraries")
  endif()
else()
  find_package(OpenCASCADE CONFIG REQUIRED)
endif()

set(FORMA_OCCT_LIBRARIES
  TKDESTEP TKXSBase TKDE TKMesh TKPrim TKBO TKFillet TKShHealing TKBool TKTopAlgo TKGeomAlgo
  TKBRep TKGeomBase TKG3d TKG2d TKMath TKernel)

add_library(forma_cad_core STATIC
  src/shape.cpp
  src/measurements.cpp
  src/face_measurements.cpp
  src/edge_measurements.cpp
  src/box_edge_selector.cpp
  src/topology_refs.cpp
  src/cylinder_topology.cpp
  src/topology_resolver.cpp
  src/reference_measurement.cpp
  src/pair_measurement.cpp
  src/primitives.cpp
  src/polygon_prism.cpp
  src/profile_prism.cpp
  src/profile_face.cpp
  src/profile_revolve.cpp
  src/section.cpp
  src/sphere.cpp
  src/cone.cpp
  src/fillet.cpp
  src/chamfer.cpp
  src/transforms.cpp
  src/rotation.cpp
  src/mirror.cpp
  src/booleans.cpp
  src/compound.cpp
  src/step_io.cpp
  src/preview.cpp)
target_include_directories(forma_cad_core PUBLIC include PRIVATE ${OpenCASCADE_INCLUDE_DIR})
target_link_directories(forma_cad_core PUBLIC "${OpenCASCADE_LIBRARY_DIR}")
target_link_libraries(forma_cad_core PUBLIC ${FORMA_OCCT_LIBRARIES})

include(CTest)
if(BUILD_TESTING)
  add_executable(forma_pair_measurement_test tests/pair_measurement_test.cpp)
  target_include_directories(forma_pair_measurement_test PRIVATE ${OpenCASCADE_INCLUDE_DIR})
  target_link_libraries(forma_pair_measurement_test PRIVATE forma_cad_core)
  add_test(NAME forma_pair_measurement_test COMMAND forma_pair_measurement_test)
  if(WIN32)
    set_tests_properties(forma_pair_measurement_test PROPERTIES
      ENVIRONMENT_MODIFICATION "PATH=path_list_prepend:${OpenCASCADE_BINARY_DIR}")
  endif()
  add_executable(forma_selected_face_section_test tests/selected_face_section_test.cpp)
  target_include_directories(forma_selected_face_section_test PRIVATE ${OpenCASCADE_INCLUDE_DIR})
  target_link_libraries(forma_selected_face_section_test PRIVATE forma_cad_core)
  add_test(NAME forma_selected_face_section_test COMMAND forma_selected_face_section_test)
  if(WIN32)
    set_tests_properties(forma_selected_face_section_test PROPERTIES
      ENVIRONMENT_MODIFICATION "PATH=path_list_prepend:${OpenCASCADE_BINARY_DIR}")
  endif()
  add_executable(forma_cylinder_topology_test tests/cylinder_topology_test.cpp)
  target_include_directories(forma_cylinder_topology_test PRIVATE ${OpenCASCADE_INCLUDE_DIR} src)
  target_link_libraries(forma_cylinder_topology_test PRIVATE forma_cad_core)
  add_test(NAME forma_cylinder_topology_test COMMAND forma_cylinder_topology_test)
  if(WIN32)
    set_tests_properties(forma_cylinder_topology_test PROPERTIES
      ENVIRONMENT_MODIFICATION "PATH=path_list_prepend:${OpenCASCADE_BINARY_DIR}")
  endif()
  add_executable(forma_reference_measurement_test tests/reference_measurement_test.cpp)
  target_include_directories(forma_reference_measurement_test PRIVATE ${OpenCASCADE_INCLUDE_DIR} src)
  target_link_libraries(forma_reference_measurement_test PRIVATE forma_cad_core)
  add_test(NAME forma_reference_measurement_test COMMAND forma_reference_measurement_test)
  if(WIN32)
    set_tests_properties(forma_reference_measurement_test PROPERTIES
      ENVIRONMENT_MODIFICATION "PATH=path_list_prepend:${OpenCASCADE_BINARY_DIR}")
  endif()
  add_executable(forma_topology_refs_test tests/topology_refs_test.cpp)
  target_include_directories(forma_topology_refs_test PRIVATE ${OpenCASCADE_INCLUDE_DIR})
  target_link_libraries(forma_topology_refs_test PRIVATE forma_cad_core)
  add_test(NAME forma_topology_refs_test COMMAND forma_topology_refs_test)
  if(WIN32)
    set_tests_properties(forma_topology_refs_test PROPERTIES
      ENVIRONMENT_MODIFICATION "PATH=path_list_prepend:${OpenCASCADE_BINARY_DIR}")
  endif()
  foreach(test_name IN ITEMS primitives booleans compound step_io)
    add_executable(forma_${test_name}_test tests/${test_name}_test.cpp)
    target_link_libraries(forma_${test_name}_test PRIVATE forma_cad_core)
    add_test(NAME forma_${test_name}_test COMMAND forma_${test_name}_test)
    if(WIN32)
      set_tests_properties(forma_${test_name}_test PROPERTIES
        ENVIRONMENT_MODIFICATION "PATH=path_list_prepend:${OpenCASCADE_BINARY_DIR}")
    endif()
  endforeach()
  add_executable(forma_fillet_test tests/fillet_test.cpp)
  target_link_libraries(forma_fillet_test PRIVATE forma_cad_core)
  add_test(NAME forma_fillet_test COMMAND forma_fillet_test)
  if(WIN32)
    set_tests_properties(forma_fillet_test PROPERTIES
      ENVIRONMENT_MODIFICATION "PATH=path_list_prepend:${OpenCASCADE_BINARY_DIR}")
  endif()
  add_executable(forma_chamfer_test tests/chamfer_test.cpp)
  target_link_libraries(forma_chamfer_test PRIVATE forma_cad_core)
  add_test(NAME forma_chamfer_test COMMAND forma_chamfer_test)
  if(WIN32)
    set_tests_properties(forma_chamfer_test PROPERTIES
      ENVIRONMENT_MODIFICATION "PATH=path_list_prepend:${OpenCASCADE_BINARY_DIR}")
  endif()
  add_executable(forma_preview_test tests/preview_test.cpp)
  target_link_libraries(forma_preview_test PRIVATE forma_cad_core)
  add_test(NAME forma_preview_test COMMAND forma_preview_test)
  if(WIN32)
    set_tests_properties(forma_preview_test PROPERTIES
      ENVIRONMENT_MODIFICATION "PATH=path_list_prepend:${OpenCASCADE_BINARY_DIR}")
  endif()
  add_executable(forma_sketch_test tests/sketch_test.cpp)
  target_link_libraries(forma_sketch_test PRIVATE forma_cad_core)
  add_test(NAME forma_sketch_test COMMAND forma_sketch_test)
  if(WIN32)
    set_tests_properties(forma_sketch_test PROPERTIES
      ENVIRONMENT_MODIFICATION "PATH=path_list_prepend:${OpenCASCADE_BINARY_DIR}")
  endif()
endif()

if(BUILD_TESTING)
  add_executable(forma_profile_test tests/profile_test.cpp)
  add_executable(forma_section_test tests/section_test.cpp)
  target_link_libraries(forma_section_test PRIVATE forma_cad_core)
  add_test(NAME forma_section_test COMMAND forma_section_test)
  target_link_libraries(forma_profile_test PRIVATE forma_cad_core)
  add_test(NAME forma_profile_test COMMAND forma_profile_test)
  if(WIN32)
    set_tests_properties(forma_profile_test PROPERTIES ENVIRONMENT_MODIFICATION "PATH=path_list_prepend:${OpenCASCADE_BINARY_DIR}")
    set_tests_properties(forma_section_test PROPERTIES ENVIRONMENT_MODIFICATION "PATH=path_list_prepend:${OpenCASCADE_BINARY_DIR}")
  endif()
endif()
