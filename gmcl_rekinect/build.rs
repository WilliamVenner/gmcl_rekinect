fn main() {
	println!("cargo:rerun-if-changed=build.rs");
	println!("cargo:rerun-if-changed=src/hax.cpp");
	println!("cargo:rerun-if-changed=src/hax.hpp");
	println!("cargo:rerun-if-changed=src/cusercmd.cpp");
	println!("cargo:rerun-if-changed=src/cusercmd.hpp");

	cc::Build::new()
		.file("src/hax.cpp")
		.file("src/cusercmd.cpp")
		.static_flag(true)
		.cargo_metadata(true)
		.cpp(true)
		.static_crt(true)
		.compile("gmcl_rekinect_cpp");
}
