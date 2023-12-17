use std::{
	borrow::Cow,
	path::{Path, PathBuf},
};

#[macro_use]
extern crate build_cfg;

#[build_cfg_main]
fn main() {
	println!("cargo:rerun-if-changed=build.rs");

	if !build_cfg!(windows) {
		return;
	}

	println!("cargo:rerun-if-env-changed=KINECTSDK20_DIR");

	println!("cargo:rerun-if-changed=src/kinect_winsdk_v2.cpp");
	println!("cargo:rerun-if-changed=src/kinect_winsdk_v2.hpp");

	let kinect_v2_sdk_path = std::env::var_os("KINECTSDK20_DIR")
		.map(PathBuf::from)
		.map(Cow::Owned)
		.unwrap_or_else(|| Cow::Borrowed(Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../lib/kinect_winsdk_v2"))));

	let kinect_v2_sdk_path = kinect_v2_sdk_path.as_ref();

	println!("cargo:rustc-link-lib=kinect20");
	println!(
		"cargo:rustc-link-search={}/lib/{}",
		kinect_v2_sdk_path.display(),
		if build_cfg!(target_pointer_width = "64") {
			"x64"
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
		.file("src/kinect_winsdk_v2.cpp")
		.include(kinect_v2_sdk_path.join("inc"))
		.compile("kinect_winsdk_v2_cpp");
}
