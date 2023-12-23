// cargo build --all && cp target/debug/gmcl_rekinect.dll "D:\Steam\steamapps\common\GarrysMod\garrysmod\lua\bin\gmcl_rekinect_win64.dll" && cp target/debug/gmcl_rekinect.dll "D:\Steam\steamapps\common\GarrysMod\garrysmod\lua\bin\gmsv_rekinect_win64.dll" && cp target/debug/rekinect_winsdk_v2.dll "D:\Steam\steamapps\common\GarrysMod\garrysmod\lua\bin\rekinect_winsdk_v2_win64.dll" && cp target/debug/rekinect_winsdk_v1.dll "D:\Steam\steamapps\common\GarrysMod\garrysmod\lua\bin\rekinect_winsdk_v1_win64.dll"

// cargo build --target i686-pc-windows-msvc --all && cp target/i686-pc-windows-msvc/debug/gmcl_rekinect.dll "D:\Steam\steamapps\common\GarrysMod\garrysmod\lua\bin\gmcl_rekinect_win32.dll" && cp target/i686-pc-windows-msvc/debug/gmcl_rekinect.dll "D:\Steam\steamapps\common\GarrysMod\garrysmod\lua\bin\gmsv_rekinect_win32.dll" && cp target/i686-pc-windows-msvc/debug/rekinect_winsdk_v2.dll "D:\Steam\steamapps\common\GarrysMod\garrysmod\lua\bin\rekinect_winsdk_v2_win32.dll" && cp target/i686-pc-windows-msvc/debug/rekinect_winsdk_v1.dll "D:\Steam\steamapps\common\GarrysMod\garrysmod\lua\bin\rekinect_winsdk_v1_win32.dll"

#![feature(array_chunks)]
#![feature(c_unwind)]
#![feature(option_get_or_insert_default)]
#![feature(thread_id_value)]
#![allow(clippy::let_and_return)]

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

	let is_client;
	{
		lua.get_global(lua_string!("CLIENT"));
		is_client = lua.get_boolean(-1);
		lua.pop();
	}

	if is_client {
		log::info!("Loaded into client state");
	} else {
		log::info!("Loaded into server state. Extended bones will be supported on this server.");
	}

	api::init(lua, is_client);
	exbones::init(lua, is_client);

	if is_client {
		// Kinect stuff is fully clientside.
		// Controlling ragdolls is done by the server.
		// Kinect bones are networked through CUserCmd.
		rekinect::init(lua);
	}
}

unsafe fn shutdown() {
	rekinect::shutdown();
}

#[gmod13_open]
unsafe fn gmod13_open(lua: gmod::lua::State) {
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
				format!("gmcl_rekinect_panic_{}.log", std::thread::current().id().as_u64()),
				format!("{:#?}", panic),
			)
			.ok();
		}
	}));
}
