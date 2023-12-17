use crate::rekinect;
use kinect::SKELETON_BONE_COUNT;
use std::ffi::c_void;

#[link(name = "gmcl_rekinect_cpp", kind = "static")]
extern "C" {
	fn set_motion_sensor_positions(lua_cusercmd: *mut c_void, positions: &[[f32; 3]; SKELETON_BONE_COUNT]);
}

#[lua_function]
unsafe fn start_command(lua: gmod::lua::State) {
	let cusercmd = lua.to_userdata(2);
	if let Some(kinect) = rekinect::state() {
		if let Some(skeleton) = &kinect.skeleton {
			set_motion_sensor_positions(cusercmd, skeleton);
			return;
		}
	}
	set_motion_sensor_positions(cusercmd, &Default::default());
}

pub unsafe fn hook(lua: gmod::lua::State) {
	lua.get_global(lua_string!("hook"));
	lua.get_field(-1, lua_string!("Add"));
	lua.push_string("StartCommand");
	lua.push_string("gmcl_rekinect");
	lua.push_function(start_command);
	lua.call(3, 0);
	lua.pop();
}
