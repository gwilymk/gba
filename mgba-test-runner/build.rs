use std::{
    env,
    path::{self, PathBuf},
};

fn find_mgba_library() -> Option<&'static str> {
    const POTENTIAL_LIBRARY_LOCATIONS: &[&str] =
        &["/usr/lib/libmgba.so.0.9", "/usr/local/lib/libmgba.so.0.9"];

    POTENTIAL_LIBRARY_LOCATIONS
        .iter()
        .find(|file_path| path::Path::new(file_path).exists())
        .copied()
}

fn main() {
    let mgba_library = find_mgba_library().expect("Need mgba 0.9 installed");

    cc::Build::new()
        .file("c/test-runner.c")
        .object(mgba_library)
        .include("c/vendor")
        .compile("test-runner");

    let bindings = bindgen::Builder::default()
        .header("c/test-runner.h")
        .generate()
        .expect("Unable to generate bindings");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("runner-bindings.rs"))
        .expect("Couldn't write bindings!");
}
