fn main() {
	println!("cargo:rerun-if-changed=build.rs");
	println!("cargo:rerun-if-changed=src/glua.cpp");
	println!("cargo:rerun-if-changed=src/glua.hpp");

	cc::Build::new()
		.file("src/glua.cpp")
		.static_flag(true)
		.cargo_metadata(true)
		.cpp(true)
		.static_crt(true)
		.compile("gm_rekinect_glua");
}
