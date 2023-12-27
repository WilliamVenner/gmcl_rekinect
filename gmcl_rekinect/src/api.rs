use crate::{cusercmd, rekinect};
use gmod::lua::{LuaReference, LUA_TNUMBER};
use kinect::{KinectSkeleton, EXTENDED_SKELETON_BONE_COUNT, SKELETON_BONE_COUNT};

static mut ORIGINAL_MOTION_SENSOR_POS: Option<LuaReference> = None;

#[lua_function]
unsafe fn start(lua: gmod::lua::State) -> i32 {
	if let Some(kinect) = rekinect::state() {
		kinect.active = true;
	}

	lua.push_boolean(true);
	1
}

#[lua_function]
unsafe fn stop(_lua: gmod::lua::State) -> i32 {
	if let Some(kinect) = rekinect::state() {
		kinect.active = false;
	}

	0
}

#[lua_function]
unsafe fn is_active(lua: gmod::lua::State) -> i32 {
	lua.push_boolean(rekinect::state().is_some_and(|kinect| kinect.active));
	1
}

#[lua_function]
unsafe fn is_available(lua: gmod::lua::State) -> i32 {
	lua.push_boolean(rekinect::state().is_some_and(|kinect| kinect.available()));
	1
}

#[lua_function]
unsafe fn motion_sensor_pos(lua: gmod::lua::State) -> i32 {
	lua.get_global(lua_string!("LocalPlayer"));
	lua.call(0, 1);

	if !lua.equal(-1, 1) {
		lua.dereference(ORIGINAL_MOTION_SENSOR_POS.unwrap());
		lua.push_value(1);
		lua.push_value(2);
		lua.call(2, 1);
		return 1;
	}

	if let Some(kinect) = rekinect::state() {
		if kinect.active && lua.lua_type(2) == LUA_TNUMBER {
			if let Ok(bone) = usize::try_from(lua.to_integer(2)) {
				let mut bone_vec = None;
				if (0..SKELETON_BONE_COUNT).contains(&bone) {
					if let KinectSkeleton::Tracked(skeleton) | KinectSkeleton::TrackedExtended(skeleton, ..) = &kinect.skeleton {
						bone_vec = Some(&skeleton.raw_bones()[bone]);
					}
				} else if (SKELETON_BONE_COUNT..SKELETON_BONE_COUNT + EXTENDED_SKELETON_BONE_COUNT).contains(&bone) {
					if let KinectSkeleton::TrackedExtended(_, skeleton) = &kinect.skeleton {
						bone_vec = Some(&skeleton.raw_bones()[SKELETON_BONE_COUNT - bone]);
					}
				}
				if let Some(bone_vec) = bone_vec {
					lua.get_global(lua_string!("Vector"));
					lua.push_number(bone_vec[0] as _);
					lua.push_number(bone_vec[1] as _);
					lua.push_number(bone_vec[2] as _);
					lua.call(3, 1);
					return 1;
				}
			}
		}
	}

	lua.get_global(lua_string!("vector_origin"));
	1
}

pub unsafe fn init(lua: gmod::lua::State) {
	lua.get_global(lua_string!("motionsensor"));
	if lua.is_nil(-1) {
		lua.pop();
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

	lua.get_global(lua_string!("FindMetaTable"));
	lua.push_string("Player");
	lua.call(1, 1);

	lua.get_field(-1, lua_string!("MotionSensorPos"));
	ORIGINAL_MOTION_SENSOR_POS = Some(lua.reference());

	lua.push_function(motion_sensor_pos);
	lua.set_field(-2, lua_string!("MotionSensorPos"));

	lua.pop();

	cusercmd::hook(lua);
}
