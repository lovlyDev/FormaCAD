use std::path::{Path, PathBuf};

pub const LIBRARIES: &[&str] = &[
    "TKDESTEP",
    "TKXSBase",
    "TKDE",
    "TKMesh",
    "TKPrim",
    "TKBO",
    "TKFillet",
    "TKShHealing",
    "TKBool",
    "TKTopAlgo",
    "TKGeomAlgo",
    "TKBRep",
    "TKGeomBase",
    "TKG3d",
    "TKG2d",
    "TKMath",
    "TKernel",
];

pub fn directories(root: &Path) -> (PathBuf, PathBuf) {
    let includes = [root.join("include/opencascade"), root.join("inc")]
        .into_iter()
        .find(|path| path.join("TopoDS_Shape.hxx").is_file())
        .expect("FORMA_OCCT_ROOT has no OCCT headers");
    let libraries = [
        root.join("lib"),
        root.join("lib64"),
        root.join("win64/vc14/lib"),
    ]
    .into_iter()
    .find(|path| LIBRARIES.iter().all(|name| has_library(path, name)))
    .expect("FORMA_OCCT_ROOT has no complete OCCT library set");
    (includes, libraries)
}

fn has_library(directory: &Path, name: &str) -> bool {
    [
        format!("{name}.lib"),
        format!("lib{name}.a"),
        format!("lib{name}.so"),
        format!("lib{name}.dylib"),
    ]
    .iter()
    .any(|filename| directory.join(filename).is_file())
}

pub fn link(libraries: &Path) {
    println!("cargo:rustc-link-search=native={}", libraries.display());
    let static_libraries = libraries.join("libTKernel.a").is_file();
    for name in LIBRARIES {
        if static_libraries {
            println!("cargo:rustc-link-lib=static={name}");
        } else {
            println!("cargo:rustc-link-lib={name}");
        }
    }
}
