use crate::{cusercmd, hax};
use gmod::lua::LUA_OK;
use std::ffi::{c_char, c_int, c_void};

static EXBONES_LUA: &str = concat!(include_str!("exbones.lua"), "\0");

pub(super) unsafe fn init(lua: gmod::lua::State) {
	let is_dedicated;
	{
		lua.get_global(lua_string!("game"));
		lua.get_field(-1, lua_string!("IsDedicated"));
		lua.call(0, 1);
		is_dedicated = lua.get_boolean(-1);
		lua.pop_n(2);
	}

	if !is_dedicated {
		// If we're on singleplayer, make the server load the module for extended bones support
		let lib = libloading::Library::new(hax::lua_shared_srv_dll_path()).expect("Failed to load lua_shared_srv");
		let lual_loadstring;
		let lua_call;
		let lua_sv;

		{
			let create_interface = lib
				.get::<*const ()>(b"CreateInterface\0")
				.expect("Failed to find CreateInterface in lua_shared_srv");

			{
				let i_lua_shared = hax::get_lua_shared(*create_interface);
				if i_lua_shared.is_null() {
					panic!("Failed to get ILuaShared");
				}
				let c_lua_interface = hax::open_lua_interface(i_lua_shared, hax::GmodLuaInterfaceRealm::Server);
				if c_lua_interface.is_null() {
					panic!("Failed to get CLuaInterface");
				}
				lua_sv = hax::get_lua_state(c_lua_interface);
				if lua_sv.is_null() {
					panic!("Failed to get lua_State");
				}
			}

			lua_call = lib
				.get::<unsafe extern "C-unwind" fn(*mut c_void, nargs: c_int, nresults: c_int)>(b"lua_call\0")
				.expect("Failed to find lua_call in lua_shared_srv");

			lual_loadstring = lib
				.get::<unsafe extern "C-unwind" fn(*mut c_void, string: *const c_char) -> c_int>(b"luaL_loadstring\0")
				.expect("Failed to find luaL_loadstring in lua_shared_srv");
		}

		let res = lual_loadstring(lua_sv, EXBONES_LUA.as_ptr() as *const _);
		if res != LUA_OK {
			panic!("luaL_loadstring exbones returned {res}");
		}
		lua_call(lua_sv, 0, 0);

		drop(lib);
	}

	#[lua_function]
	unsafe fn extended_bones_supported_callback(lua: gmod::lua::State) {
		let extended_bones_supported = lua.get_boolean(1);
		cusercmd::extended_bones_supported(extended_bones_supported);

		lua.push_nil();
		lua.set_global(lua_string!("gmcl_rekinect_extended_bones_supported_callback"));
	}

	lua.push_function(extended_bones_supported_callback);
	lua.set_global(lua_string!("gmcl_rekinect_extended_bones_supported_callback"));

	lua.load_string(EXBONES_LUA.as_ptr() as *const _).unwrap();
	lua.call(0, 0);
}
