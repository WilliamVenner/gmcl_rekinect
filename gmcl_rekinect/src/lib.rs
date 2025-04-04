// cargo build --all && cp target/debug/gmcl_rekinect.dll "D:\Steam\steamapps\common\GarrysMod\garrysmod\lua\bin\gmcl_rekinect_win64.dll" && cp target/debug/rekinect_winsdk_v2.dll "D:\Steam\steamapps\common\GarrysMod\garrysmod\lua\bin\rekinect_winsdk_v2_win64.dll" && cp target/debug/rekinect_winsdk_v1.dll "D:\Steam\steamapps\common\GarrysMod\garrysmod\lua\bin\rekinect_winsdk_v1_win64.dll"

// cargo build --target i686-pc-windows-msvc --all && cp target/i686-pc-windows-msvc/debug/gmcl_rekinect.dll "D:\Steam\steamapps\common\GarrysMod\garrysmod\lua\bin\gmcl_rekinect_win32.dll" && cp target/i686-pc-windows-msvc/debug/rekinect_winsdk_v2.dll "D:\Steam\steamapps\common\GarrysMod\garrysmod\lua\bin\rekinect_winsdk_v2_win32.dll" && cp target/i686-pc-windows-msvc/debug/rekinect_winsdk_v1.dll "D:\Steam\steamapps\common\GarrysMod\garrysmod\lua\bin\rekinect_winsdk_v1_win32.dll"

#![allow(clippy::let_and_return)]

use std::borrow::Cow;

#[macro_use]
extern crate gmod;

mod api;
mod cusercmd;
mod exbones;
mod hax;
mod logging;
mod rekinect;

static mut GMOD13_OPEN: bool = false;

unsafe fn init(lua: gmod::lua::State) {
	log::info!(concat!("gmcl_rekinect v", env!("CARGO_PKG_VERSION"), " loaded!"));

	lua_stack_guard!(lua => {
		api::init(lua);
	});
	lua_stack_guard!(lua => {
		exbones::init(lua);
	});
	lua_stack_guard!(lua => {
		rekinect::init(lua);
	});
}

unsafe fn shutdown() {
	rekinect::shutdown();
}

#[gmod13_open]
unsafe fn gmod13_open(lua: gmod::lua::State) {
	let is_server;
	{
		lua.get_global(lua_string!("SERVER"));
		is_server = lua.get_boolean(-1);
		lua.pop();
	}

	// If we're on the server, don't do anything.
	if is_server {
		log::info!("gmcl_rekinect is a clientside module, and does nothing on the server.");
		return;
	}

	// If we're already injected, don't do anything.
	if rekinect::already_initialized() {
		return;
	}

	GMOD13_OPEN = true;

	logging::init_for_binary_module();
	hax::binary_module_init(lua);
	init(lua);
}

#[gmod13_close]
unsafe fn gmod13_close(_lua: gmod::lua::State) {
	if GMOD13_OPEN {
		shutdown();
	}
}

// Support for DLL injecting
#[ctor::ctor]
fn ctor() {
	set_panic_handler();

	// If we're already injected, don't do anything.
	if unsafe { rekinect::already_initialized() } {
		return;
	}

	unsafe { hax::init() };
}

fn set_panic_handler() {
	std::panic::set_hook(Box::new(move |panic: &std::panic::PanicInfo<'_>| {
		if let Some(lua) = hax::lua_state() {
			unsafe {
				lua.get_global(lua_string!("ErrorNoHalt"));
				if !lua.is_nil(-1) {
					lua.push_string(&format!("gmcl_rekinect panic: {:#?}\n", panic));
					lua.call(1, 0);
				} else {
					lua.pop();
				}
			}
		} else {
			std::fs::write(
				format!(
					"gmcl_rekinect_panic_{}.log",
					Some(format!("{:?}", std::thread::current().id()).replace(|char| !char::is_ascii_digit(&char), ""))
						.filter(|s| !s.is_empty())
						.map(Cow::Owned)
						.unwrap_or_else(|| Cow::Borrowed("unknown"))
				),
				format!("{:#?}", panic),
			)
			.ok();
		}
	}));
}
