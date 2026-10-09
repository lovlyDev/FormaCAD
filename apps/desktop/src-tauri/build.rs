#[cfg(feature = "native-occt")]
#[path = "build/occt.rs"]
mod occt;

fn main() {
    #[cfg(feature = "native-occt")]
    {
        let root = std::path::PathBuf::from(
            std::env::var("FORMA_OCCT_ROOT")
                .expect("FORMA_OCCT_ROOT must point to an installed OCCT triplet"),
        );
        let (includes, libraries) = occt::directories(&root);
        cxx_build::bridge("src/native.rs")
            .file("../../../cad-core/src/shape.cpp")
            .file("../../../cad-core/src/measurements.cpp")
            .file("../../../cad-core/src/face_measurements.cpp")
            .file("../../../cad-core/src/edge_measurements.cpp")
            .file("../../../cad-core/src/box_edge_selector.cpp")
            .file("../../../cad-core/src/topology_refs.cpp")
            .file("../../../cad-core/src/cylinder_topology.cpp")
            .file("../../../cad-core/src/topology_resolver.cpp")
            .file("../../../cad-core/src/reference_measurement.cpp")
            .file("../../../cad-core/src/pair_measurement.cpp")
            .file("../../../cad-core/src/primitives.cpp")
            .file("../../../cad-core/src/polygon_prism.cpp")
            .file("../../../cad-core/src/profile_prism.cpp")
            .file("../../../cad-core/src/profile_face.cpp")
            .file("../../../cad-core/src/profile_revolve.cpp")
            .file("../../../cad-core/src/section.cpp")
            .file("../../../cad-core/src/sphere.cpp")
            .file("../../../cad-core/src/cone.cpp")
            .file("../../../cad-core/src/fillet.cpp")
            .file("../../../cad-core/src/chamfer.cpp")
            .file("../../../cad-core/src/transforms.cpp")
            .file("../../../cad-core/src/rotation.cpp")
            .file("../../../cad-core/src/mirror.cpp")
            .file("../../../cad-core/src/booleans.cpp")
            .file("../../../cad-core/src/compound.cpp")
            .file("../../../cad-core/src/step_io.cpp")
            .file("../../../cad-core/src/preview.cpp")
            .include("../../..")
            .include("../../../cad-core/include")
            .include(&includes)
            .std("c++20")
            .flag_if_supported("/EHsc")
            .compile("forma_native_bridge");
        occt::link(&libraries);
        println!("cargo:rerun-if-changed=src/native.rs");
        println!("cargo:rerun-if-changed=build/occt.rs");
        println!("cargo:rerun-if-changed=../../../cad-core/src");
        println!("cargo:rerun-if-changed=../../../cad-core/include/forma_core.hpp");
        println!("cargo:rerun-if-env-changed=FORMA_OCCT_ROOT");
    }
    tauri_build::build()
}
