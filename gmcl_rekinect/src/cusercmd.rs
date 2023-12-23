use crate::rekinect;
use kinect::{KinectSkeleton, KinectSkeletonRawBones};
use std::ffi::c_void;

static mut SEND_EXTENDED_BONES: bool = false;

#[link(name = "gmcl_rekinect_cpp", kind = "static")]
extern "C" {
	fn set_motion_sensor_positions(lua_cusercmd: *mut c_void, positions: *const [f32; 3]);
}

#[lua_function]
unsafe fn start_command(lua: gmod::lua::State) {
	static mut EXTENDED_BONES_CLEARED: bool = true;

	let cusercmd = lua.to_userdata(2);

	let cmd_number;
	{
		lua.get_field(2, lua_string!("CommandNumber"));
		lua.push_value(2);
		lua.call(1, 1);
		cmd_number = lua.to_integer(-1);
		lua.pop();
	}

	if let Some(kinect) = rekinect::state() {
		if let (1.., true, true, KinectSkeleton::TrackedExtended(.., extended)) = (cmd_number, SEND_EXTENDED_BONES, kinect.active, &kinect.skeleton) {
			EXTENDED_BONES_CLEARED = false;

			lua.get_global(lua_string!("net"));

			lua.push_value(-1);
			lua.get_field(-1, lua_string!("Start"));
			lua.push_string("gmcl_rekinect_extended_bones");
			lua.push_boolean(true);
			lua.call(2, 0);

			lua.push_value(-1);
			lua.get_field(-1, lua_string!("WriteUInt"));
			lua.push_integer(cmd_number);
			lua.push_integer(32);
			lua.call(2, 0);

			lua.push_value(-1);
			lua.get_field(-1, lua_string!("WriteBool"));
			lua.push_boolean(false);
			lua.call(1, 0);

			lua.push_value(-1);
			lua.get_field(-1, lua_string!("WriteVector"));

			lua.get_global(lua_string!("Vector"));

			for bone in extended.raw_bones() {
				lua.push_value(-2);

				lua.push_value(-2);
				lua.push_number(bone[0] as _);
				lua.push_number(bone[1] as _);
				lua.push_number(bone[2] as _);
				lua.call(3, 1);

				lua.call(1, 0);
			}
			lua.pop_n(2);

			lua.get_field(-1, lua_string!("SendToServer"));
			lua.call(0, 0);
		}

		if let KinectSkeleton::Tracked(skeleton) | KinectSkeleton::TrackedExtended(skeleton, ..) = &kinect.skeleton {
			set_motion_sensor_positions(cusercmd, skeleton.raw_bones().as_ptr());
			return;
		}
	}

	set_motion_sensor_positions(cusercmd, KinectSkeletonRawBones::default().as_ptr());

	if cmd_number != 0 && SEND_EXTENDED_BONES && !EXTENDED_BONES_CLEARED {
		// Don't spam the server with zeroed extended bones
		EXTENDED_BONES_CLEARED = true;

		lua.get_global(lua_string!("net"));

		lua.push_value(-1);
		lua.get_field(-1, lua_string!("Start"));
		lua.push_string("gmcl_rekinect_extended_bones");
		lua.call(1, 0);

		lua.push_value(-1);
		lua.get_field(-1, lua_string!("WriteUInt"));
		lua.push_integer(cmd_number);
		lua.push_integer(32);
		lua.call(2, 0);

		lua.push_value(-1);
		lua.get_field(-1, lua_string!("WriteBool"));
		lua.push_boolean(true);
		lua.call(1, 0);

		lua.get_field(-1, lua_string!("SendToServer"));
		lua.call(0, 0);
	}
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

pub unsafe fn extended_bones_supported(is_supported: bool) {
	SEND_EXTENDED_BONES = is_supported;
}
