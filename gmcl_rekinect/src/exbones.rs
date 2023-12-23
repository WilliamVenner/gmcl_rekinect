use crate::{cusercmd, hax};
use std::ffi::{c_char, c_void};

pub(super) unsafe fn init(lua: gmod::lua::State, is_client: bool) {
	if is_client {
		let is_dedicated;
		{
			lua.get_global(lua_string!("game"));
			lua.get_field(-1, lua_string!("IsDedicated"));
			lua.call(0, 1);
			is_dedicated = lua.get_boolean(-1);
			lua.pop();
		}

		if !is_dedicated {
			let is_sv_installed;
			{
				// If we're on singleplayer, make the server load the module for extended bones support
				let lib = libloading::Library::new(hax::lua_shared_srv_dll_path()).expect("Failed to load lua_shared_srv");
				let lual_loadstring;
				let lua_call;
				let lua_settop;
				let lua_toboolean;
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
						lua_sv = hax::get_lua_state(c_lua_interface);
					}

					lua_call = lib
						.get::<unsafe extern "C-unwind" fn(*mut c_void, nargs: i32, nresults: i32)>(b"lua_call\0")
						.expect("Failed to find lua_call in lua_shared_srv");
					lua_settop = lib
						.get::<unsafe extern "C-unwind" fn(*mut c_void, top: i32)>(b"lua_settop\0")
						.expect("Failed to find lua_settop in lua_shared_srv");
					lua_toboolean = lib
						.get::<unsafe extern "C-unwind" fn(*mut c_void, index: i32) -> bool>(b"lua_toboolean\0")
						.expect("Failed to find lua_toboolean in lua_shared_srv");
					lual_loadstring = lib
						.get::<unsafe extern "C-unwind" fn(*mut c_void, string: *const c_char)>(b"luaL_loadstring\0")
						.expect("Failed to find luaL_loadstring in lua_shared_srv");
				}

				lual_loadstring(
					lua_sv,
					concat!(
						r#"local installed = util.IsBinaryModuleInstalled("rekinect") if installed then require("rekinect") end return installed"#,
						"\0"
					)
					.as_ptr() as *const _,
				);
				lua_call(lua_sv, 0, 1);
				is_sv_installed = lua_toboolean(lua_sv, -1);
				lua_settop(lua_sv, -2);
			};

			if !is_sv_installed {
				lua.get_global(lua_string!("chat"));
				lua.get_field(-1, lua_string!("AddText"));

				lua.get_global(lua_string!("Color"));
				lua.push_integer(255);
				lua.push_integer(0);
				lua.push_integer(0);
				lua.call(3, 1);

				lua.push_string(
					"Failed to load gmcl_rekinect on server because gmsv_rekinect is missing! Xbox One Kinect extended bones support won't work.",
				);

				lua.call(2, 0);
				lua.pop();
			}
		}
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

	lua.load_string(concat!(include_str!("exbones.lua"), "\0").as_ptr() as *const _).unwrap();
	lua.call(0, 0);
}
