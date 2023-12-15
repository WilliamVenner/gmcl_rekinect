#[macro_use]
extern crate build_cfg;

#[build_cfg_main]
fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    /*println!("cargo:rerun-if-changed=src/gm_kinect_libfreenect.cpp");
    println!("cargo:rerun-if-changed=src/gm_kinect_libfreenect.hpp");

    cc::Build::new()
        .cpp(true)
        .cargo_metadata(true)
        .static_flag(true)
        .file("src/gm_kinect_libfreenect.cpp")
        .compile("gm_kinect_libfreenect");*/
}
