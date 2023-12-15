use std::path::PathBuf;

#[macro_use]
extern crate build_cfg;

#[build_cfg_main]
fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    if !build_cfg!(windows) {
        return;
    }

    println!("cargo:rerun-if-env-changed=KINECTSDK10_DIR");

    println!("cargo:rerun-if-changed=src/kinect_winsdk_v1.cpp");
    println!("cargo:rerun-if-changed=src/kinect_winsdk_v1.hpp");

    let kinect_v1_sdk_path =
        PathBuf::from(std::env::var_os("KINECTSDK10_DIR").expect("KINECTSDK10_DIR not set"));

    println!("cargo:rustc-link-lib=kinect10");
    println!(
        "cargo:rustc-link-search={}/lib/{}",
        kinect_v1_sdk_path.display(),
        if build_cfg!(target_pointer_width = "64") {
            "amd64"
        } else if build_cfg!(target_pointer_width = "32") {
            "x86"
        } else {
            panic!("unsupported target_pointer_width")
        }
    );

    cc::Build::new()
        .cpp(true)
        .cargo_metadata(true)
        .static_flag(true)
        .file("src/kinect_winsdk_v1.cpp")
        .include(kinect_v1_sdk_path.join("inc"))
        .compile("kinect_winsdk_v1_cpp");
}
