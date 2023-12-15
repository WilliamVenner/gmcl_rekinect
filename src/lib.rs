// cargo build --all && cp target/debug/gm_rekinect.dll "D:\Steam\steamapps\common\GarrysMod\garrysmod\lua\bin\gmcl_rekinect_win64.dll" && cp target/debug/gm_rekinect.dll "D:\Steam\steamapps\common\GarrysMod\garrysmod\lua\bin\gmsv_rekinect_win64.dll" && cp target/debug/gm_rekinect_winsdk_v2.dll "D:\Steam\steamapps\common\GarrysMod\garrysmod\lua\bin\gm_rekinect_winsdk_v2.dll" && cp target/debug/gm_rekinect_winsdk_v1.dll "D:\Steam\steamapps\common\GarrysMod\garrysmod\lua\bin\gm_rekinect_winsdk_v1.dll"
#![feature(array_chunks)]
#![feature(c_unwind)]
#![feature(option_get_or_insert_default)]
#![feature(thread_id_value)]

#[macro_use]
extern crate gmod;

use kinect::*;
use std::{
	borrow::Cow,
	cell::Cell,
	ffi::{c_char, c_int, c_void, OsString},
	fs::OpenOptions,
	mem::{size_of, ManuallyDrop},
	path::Path,
};

thread_local! {
	static LUA_STATE: Cell<Option<gmod::lua::State>> = Cell::new(None);
}

struct Logger;
impl log::Log for Logger {
	fn log(&self, record: &log::Record) {
		let Some(lua) = LUA_STATE.get() else { return };
		unsafe {
			lua.get_global(lua_string!("print"));
			lua.push_string(&if record.level() != log::Level::Info {
				format!("gm_rekinect: [{}] {}", record.level(), record.args())
			} else {
				format!("gm_rekinect: {}", record.args())
			});
			lua.call(1, 0);
		}
	}

	#[inline]
	fn enabled(&self, metadata: &log::Metadata) -> bool {
		metadata.level() <= log::Level::Info
	}

	fn flush(&self) {}
}

#[repr(i32)]
enum GmodLuaInterfaceRealm {
	Client = 0,
	Server = 1,
	Menu = 2,
}

#[link(name = "gm_rekinect_glua", kind = "static")]
extern "C" {
	fn ctor_lua_state(create_interface: *const (), realm: GmodLuaInterfaceRealm) -> *mut std::ffi::c_void;
}

static mut KINECT: Option<KinectState> = None;

const MMAP_FILE_SIZE: u64 =
	(size_of::<u8>() + size_of::<u8>() + size_of::<u8>() + size_of::<u16>() + (size_of::<[f32; 3]>() * kinect::SKELETON_BONE_COUNT)) as u64;

const MMAP_KINECT_SKELETON_NONE: u8 = 0;
const MMAP_KINECT_SKELETON_TRACKED: u8 = 1;

const MMAP_SHUTDOWN: usize = 0;
const MMAP_ACTIVE: usize = 1;
const MMAP_SYNC: std::ops::Range<usize> = 2..4;
const MMAP_SKELETON: usize = 4;
const MMAP_SKELETON_BONES: std::ops::Range<usize> = 5..MMAP_FILE_SIZE as usize;

struct KinectState {
	mmap: memmap::MmapMut,
	skeleton: Option<[[f32; 3]; kinect::SKELETON_BONE_COUNT]>,
	kind: KinectStateKind,
}
impl KinectState {
	fn new() -> Result<Self, std::io::Error> {
		// We're a client if garrysmod/cache/gm_rekinect/klient_pid.dat exists
		let mmap_name = OsString::from(format!("kinect_{}", std::process::id()));
		let client = 'client: {
			if let Ok(dir) = std::fs::read_dir("garrysmod/cache/gm_rekinect") {
				for entry in dir.flatten() {
					let entry = entry.path();
					if entry.file_name() == Some(mmap_name.as_os_str()) {
						break 'client true;
					}
				}
			}
			break 'client false;
		};
		if !client {
			// Clean up old mmaps
			std::fs::remove_dir_all("garrysmod/cache/gm_rekinect").ok();
		}

		std::fs::create_dir_all("garrysmod/cache/gm_rekinect")?;

		let mmap_path = Path::new("garrysmod/cache/gm_rekinect").join(mmap_name);

		let f = OpenOptions::new().write(true).read(true).truncate(false).create(true).open(mmap_path)?;

		f.set_len(MMAP_FILE_SIZE)?;

		let mut mmap = unsafe { memmap::MmapMut::map_mut(&f)? };

		if client {
			log::info!("client connected to mmap");

			let mut client = Self {
				mmap,
				skeleton: None,
				kind: KinectStateKind::Client { sync: None },
			};

			client.update();

			Ok(client)
		} else {
			log::info!("mmap server opened");

			let inner = Kinect::new()?;

			mmap.fill(0);
			mmap.flush().ok();

			Ok(Self {
				mmap,
				skeleton: None,
				kind: KinectStateKind::Server {
					inner: ManuallyDrop::new(inner),
					sync: 0,
				},
			})
		}
	}

	fn update(&mut self) {
		match &mut self.kind {
			KinectStateKind::Server { inner, sync } => {
				if self.mmap[MMAP_ACTIVE] != 1 {
					return;
				}

				let Some(update) = inner.poll() else {
					return;
				};

				*sync = sync.wrapping_add(1);
				self.mmap[MMAP_SYNC].copy_from_slice(&u16::to_ne_bytes(*sync));

				if let KinectSkeleton::Tracked(pos) = update {
					self.mmap[MMAP_SKELETON] = MMAP_KINECT_SKELETON_TRACKED;

					let skeleton = self.skeleton.get_or_insert_default();

					for ((vec, mmap), skeleton) in pos
						.raw_bones()
						.iter()
						.zip(self.mmap[MMAP_SKELETON_BONES].array_chunks_mut::<{ size_of::<[f32; 3]>() }>())
						.zip(skeleton.iter_mut())
					{
						mmap[0..4].copy_from_slice(&f32::to_ne_bytes(vec[0]));
						mmap[4..8].copy_from_slice(&f32::to_ne_bytes(vec[1]));
						mmap[8..12].copy_from_slice(&f32::to_ne_bytes(vec[2]));

						*skeleton = *vec;
					}

					self.mmap
						.flush_range(MMAP_SYNC.start, (MMAP_SYNC.start..MMAP_SKELETON_BONES.end).len())
						.ok();
				} else {
					self.mmap[MMAP_SKELETON] = MMAP_KINECT_SKELETON_NONE;
					self.mmap.flush_range(MMAP_SYNC.start, (MMAP_SYNC.start..MMAP_SKELETON).len()).ok();

					self.skeleton = None;
				}
			}

			KinectStateKind::Client { sync } => {
				let shutdown = self.mmap[MMAP_SHUTDOWN];
				if shutdown == 1 {
					log::info!("trying to promote to server");

					// Promote to server
					if let Ok(inner) = Kinect::new() {
						if core::mem::replace(&mut self.mmap[MMAP_SHUTDOWN], 0) != 1 {
							return self.update();
						}

						if self.mmap.flush_range(MMAP_SHUTDOWN, 1).is_ok() {
							self.kind = KinectStateKind::Server {
								inner: ManuallyDrop::new(inner),
								sync: sync.unwrap_or(0),
							};

							log::info!("promoted to server");

							return self.update();
						}
					}
					return;
				}

				let new_sync = Some(u16::from_ne_bytes(self.mmap[MMAP_SYNC].try_into().unwrap()));
				if new_sync == core::mem::replace(sync, new_sync) {
					// No changes
					return;
				}

				match self.mmap[MMAP_SKELETON] {
					MMAP_KINECT_SKELETON_NONE => {
						self.skeleton = None;
					}

					MMAP_KINECT_SKELETON_TRACKED => {
						let skeleton = self.skeleton.get_or_insert_default();
						for (bone, skeleton) in self.mmap[MMAP_SKELETON_BONES]
							.array_chunks::<{ size_of::<[f32; 3]>() }>()
							.zip(skeleton.iter_mut())
						{
							*skeleton = [
								f32::from_ne_bytes(bone[0..4].try_into().unwrap()),
								f32::from_ne_bytes(bone[4..8].try_into().unwrap()),
								f32::from_ne_bytes(bone[8..12].try_into().unwrap()),
							];
						}
					}

					_ => unreachable!(),
				}
			}
		}
	}

	fn active(&self) -> bool {
		self.mmap[MMAP_ACTIVE] == 1
	}

	fn set_active(&mut self, active: bool) {
		self.mmap[MMAP_ACTIVE] = active as u8;
		self.mmap.flush_range(MMAP_ACTIVE, 1).ok();
	}
}
impl Drop for KinectState {
	fn drop(&mut self) {
		if let KinectStateKind::Server { inner, .. } = &mut self.kind {
			// Shut down Kinect
			unsafe { ManuallyDrop::drop(inner) };

			// Mark shutdown byte
			self.mmap[MMAP_SHUTDOWN] = 1;
			self.mmap.flush_range(MMAP_SHUTDOWN, 1).ok();
		}
	}
}

enum KinectStateKind {
	Server { inner: ManuallyDrop<Kinect>, sync: u16 },

	Client { sync: Option<u16> },
}

#[lua_function]
unsafe fn poll(_lua: gmod::lua::State) {
	if let Some(kinect) = &mut KINECT {
		kinect.update();
	}
}

#[lua_function]
unsafe fn start(_lua: gmod::lua::State) -> i32 {
	if let Some(kinect) = KINECT.as_mut() {
		kinect.set_active(true);
	}

	1
}

#[lua_function]
unsafe fn stop(_lua: gmod::lua::State) -> i32 {
	if let Some(kinect) = KINECT.as_mut() {
		kinect.set_active(false);
	}

	0
}

#[lua_function]
unsafe fn is_active(lua: gmod::lua::State) -> i32 {
	lua.push_boolean(KINECT.as_ref().map(|kinect| kinect.active()).unwrap_or(false));

	1
}

#[lua_function]
unsafe fn is_available(lua: gmod::lua::State) -> i32 {
	lua.push_boolean(KINECT.is_some());
	1
}

#[lua_function]
unsafe fn get_table(lua: gmod::lua::State) -> i32 {
	lua.create_table(kinect::SKELETON_BONE_COUNT as _, 0);

	if let Some(kinect) = &mut KINECT {
		kinect.update();

		if let Some(skeleton) = &kinect.skeleton {
			// TODO replace with gmod vector

			lua.get_global(lua_string!("Vector"));

			for (i, pos) in skeleton.iter().enumerate() {
				lua.push_value(-1);
				lua.push_number(pos[0] as _);
				lua.push_number(pos[1] as _);
				lua.push_number(pos[2] as _);
				lua.call(3, 1);
				lua.raw_seti(-3, i as _);
			}

			lua.pop();

			return 1;
		}
	}

	// Push zeroed table
	lua.get_global(lua_string!("vector_origin"));
	for i in 0..kinect::SKELETON_BONE_COUNT as i32 {
		lua.push_value(-1);
		lua.raw_seti(-3, i);
	}
	lua.pop();

	1
}

#[lua_function]
unsafe fn player_motion_sensor_pos(lua: gmod::lua::State) -> i32 {
	let pos = if let Some(kinect) = &mut KINECT {
		kinect.update();

		if let Some(skeleton) = &mut kinect.skeleton {
			usize::try_from(lua.to_integer(2)).ok().and_then(|idx| skeleton.get(idx))
		} else {
			None
		}
	} else {
		None
	};

	// TODO other players
	// TOOD use gmod vector
	lua.get_global(lua_string!("Vector"));
	if let Some(pos) = pos {
		lua.push_number(pos[0] as _);
		lua.push_number(pos[1] as _);
		lua.push_number(pos[2] as _);
	} else {
		lua.push_integer(0);
		lua.push_integer(0);
		lua.push_integer(0);
	}
	lua.call(3, 1);

	1
}

#[lua_function]
unsafe fn get_colour_material(lua: gmod::lua::State) -> i32 {
	// TODO
	lua.get_global(lua_string!("Material"));
	lua.push_string("pp/colour");
	lua.call(1, 1);
	1
}

#[gmod13_open]
fn gmod13_open(lua: gmod::lua::State) {
	set_panic_handler();

	LUA_STATE.set(Some(lua));

	log::set_logger(&Logger).ok();
	log::set_max_level(log::LevelFilter::Info);

	unsafe {
		log::info!("gm_rekinect loaded!");

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

		lua.push_string("GetTable");
		lua.push_function(get_table);
		lua.set_table(-3);

		lua.push_string("GetColourMaterial");
		lua.push_function(get_colour_material);
		lua.set_table(-3);

		lua.pop();

		lua.get_global(lua_string!("FindMetaTable"));
		lua.push_string("Player");
		lua.call(1, 1);

		if !lua.is_nil(-1) {
			lua.push_string("MotionSensorPos");
			lua.push_function(player_motion_sensor_pos);
			lua.set_table(-3);
		}

		lua.pop();

		match KinectState::new() {
			Ok(kinect) => {
				KINECT = Some(kinect);

				lua.get_global(lua_string!("hook"));
				lua.get_field(-1, lua_string!("Add"));
				lua.push_string("Think");
				lua.push_string("gm_rekinect");
				lua.push_function(poll);
				lua.call(3, 0);
				lua.pop();
			}

			Err(err) => {
				log::error!("{err:?}");
			}
		}
	}
}

#[gmod13_close]
fn gmod13_close(_lua: gmod::lua::State) {
	unsafe { KINECT = None };
}

// Support for DLL injecting
#[ctor::ctor]
fn ctor() {
	set_panic_handler();

	unsafe {
		let lib = {
			#[cfg(windows)]
			{
				libloading::os::windows::Library::open_already_loaded("lua_shared")
			}
			#[cfg(unix)]
			{
				libloading::os::unix::Library::open(Some("lua_shared_srv"), libc::RTLD_NOLOAD)
					.or_else(|_| libloading::os::unix::Library::open(Some("lua_shared"), libc::RTLD_NOLOAD))
			}
		};

		let lib = lib.expect("Failed to find lua_shared");

		let gmod_load_binary_module = lib
			.get::<extern "C" fn(*mut c_void, *const c_char) -> c_int>(b"GMOD_LoadBinaryModule")
			.expect("Failed to find GMOD_LoadBinaryModule in lua_shared");

		let create_interface = lib
			.get::<*const ()>(b"CreateInterface")
			.expect("Failed to find CreateInterface in lua_shared");

		let lua_type = lib
			.get::<unsafe extern "C-unwind" fn(state: *mut c_void, index: c_int) -> c_int>(b"lua_type")
			.expect("Failed to find lua_type in lua_shared");

		let lua_gettop = lib
			.get::<unsafe extern "C-unwind" fn(state: *mut c_void) -> c_int>(b"lua_gettop")
			.expect("Failed to find lua_gettop in lua_shared");

		let suffix = match () {
			_ if cfg!(all(target_pointer_width = "64", windows)) => "win64",
			_ if cfg!(all(target_pointer_width = "32", windows)) => "win32",
			_ if cfg!(all(target_pointer_width = "64", target_os = "linux")) => "linux64",
			_ if cfg!(all(target_pointer_width = "32", target_os = "linux")) => "linux",
			_ if cfg!(target_os = "macos") => "osx",
			_ => panic!("Unsupported platform"),
		};

		let cl = ctor_lua_state(*create_interface, GmodLuaInterfaceRealm::Client);
		let sv = ctor_lua_state(*create_interface, GmodLuaInterfaceRealm::Server);
		let mn = ctor_lua_state(*create_interface, GmodLuaInterfaceRealm::Menu);

		// First check if we're already being loaded by GMOD_LoadBinaryModule
		for lua in [cl, sv, mn] {
			if lua.is_null() {
				// This Lua state isn't active, ignore it
				continue;
			}

			if lua_type(lua, 1) == gmod::lua::LUA_TSTRING {
				// We're already being loaded by GMOD_LoadBinaryModule, bail
				// This detection really sucks, but it works
				return;
			}
		}

		for (lua, prefix) in [(cl, "gmcl"), (sv, "gmsv"), (mn, "gmsv")] {
			if lua.is_null() {
				continue;
			}

			let path = OsString::from(format!("garrysmod/lua/bin/{prefix}_rekinect_{suffix}.dll\0"));

			// FIXME
		}

		std::mem::forget(lib);
	}
}

fn set_panic_handler() {
	std::panic::set_hook(Box::new(move |panic| {
		let path = if let Some(lua) = LUA_STATE.get() {
			unsafe {
				lua.get_global(lua_string!("ErrorNoHalt"));
				lua.push_string(&format!("Kinect panic: {:#?}\n", panic));
				lua.call(1, 0);
			}
			Cow::Borrowed("gm_rekinect_panic.txt")
		} else {
			Cow::Owned(format!("gm_rekinect_panic_{}.txt", std::thread::current().id().as_u64()))
		};

		std::fs::write(path.as_ref(), format!("{:#?}", panic)).ok();
	}));
}
