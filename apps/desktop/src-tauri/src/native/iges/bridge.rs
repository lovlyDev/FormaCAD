use crate::native::{NativeError, Solid};
use std::path::Path;
#[cxx::bridge(namespace = "forma")]
mod ffi {
    unsafe extern "C++" {
        include!("cad-core/include/forma_iges.hpp");
        type Shape = super::super::super::ffi::Shape;
        fn read_iges(path: &CxxString) -> UniquePtr<Shape>;
        fn write_iges(source: &Shape, path: &CxxString, error: Pin<&mut CxxString>) -> bool;
    }
}
impl Solid {
    pub fn read_iges(path: &Path) -> Result<Self, NativeError> {
        let name = path.to_str().ok_or_else(|| NativeError {
            code: "IGES_IO_FAILED".into(),
            detail: "Exchange path is not UTF-8".into(),
        })?;
        cxx::let_cxx_string!(filename = name);
        Self::checked(ffi::read_iges(&filename))
    }
    pub fn write_iges(&self, path: &Path) -> Result<(), NativeError> {
        let name = path.to_str().ok_or_else(|| NativeError {
            code: "IGES_IO_FAILED".into(),
            detail: "Exchange path is not UTF-8".into(),
        })?;
        cxx::let_cxx_string!(filename = name);
        cxx::let_cxx_string!(error = "");
        if ffi::write_iges(self.as_ref(), &filename, error.as_mut()) {
            Ok(())
        } else {
            Err(NativeError {
                code: error.to_string_lossy().into_owned(),
                detail: "IGES export or round-trip validation failed".into(),
            })
        }
    }
}
