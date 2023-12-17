use crate::{cusercmd, rekinect};

#[lua_function]
unsafe fn start(_lua: gmod::lua::State) -> i32 {
	if let Some(kinect) = rekinect::state() {
		kinect.set_active(true);
	}

	1
}

#[lua_function]
unsafe fn stop(_lua: gmod::lua::State) -> i32 {
	if let Some(kinect) = rekinect::state() {
		kinect.set_active(false);
	}

	0
}

#[lua_function]
unsafe fn is_active(lua: gmod::lua::State) -> i32 {
	lua.push_boolean(rekinect::state().map(|kinect| kinect.active()).unwrap_or(false));

	1
}

#[lua_function]
unsafe fn is_available(lua: gmod::lua::State) -> i32 {
	lua.push_boolean(rekinect::state().is_some());
	1
}

pub unsafe fn init(lua: gmod::lua::State) {
	log::info!("gmcl_rekinect loaded!");

	{
		lua.get_global(lua_string!("motionsensor"));
		if lua.is_nil(-1) {
			lua.create_table(0, 0);
			lua.set_global(lua_string!("motionsensor"));
			lua.get_global(lua_string!("motionsensor"));
		}

		lua.push_string("Start");
		lua.push_function(start);
		lua.set_table(-3);

		lua.push_string("Stop");
		lua.push_function(stop);
		lua.set_table(-3);

		lua.push_string("IsActive");
		lua.push_function(is_active);
		lua.set_table(-3);

		lua.push_string("IsAvailable");
		lua.push_function(is_available);
		lua.set_table(-3);

		lua.pop();
	}

	lua.get_global(lua_string!("CLIENT"));
	if !lua.is_nil(-1) {
		cusercmd::hook(lua);
	}
	lua.pop();
}
