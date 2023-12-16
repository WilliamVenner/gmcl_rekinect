// cargo build --all && cp target/debug/gm_rekinect.dll "D:\Steam\steamapps\common\GarrysMod\garrysmod\lua\bin\gmcl_rekinect_win64.dll" && cp target/debug/gm_rekinect.dll "D:\Steam\steamapps\common\GarrysMod\garrysmod\lua\bin\gmsv_rekinect_win64.dll" && cp target/debug/gm_rekinect_winsdk_v2.dll "D:\Steam\steamapps\common\GarrysMod\garrysmod\lua\bin\gm_rekinect_winsdk_v2.dll" && cp target/debug/gm_rekinect_winsdk_v1.dll "D:\Steam\steamapps\common\GarrysMod\garrysmod\lua\bin\gm_rekinect_winsdk_v1.dll"
#![feature(array_chunks)]
#![feature(c_unwind)]
#![feature(option_get_or_insert_default)]
#![feature(thread_id_value)]

#[macro_use]
extern crate gmod;

use kinect::*;
use std::{
	collections::HashMap,
	ffi::{c_void, OsString},
	fs::OpenOptions,
	mem::{size_of, ManuallyDrop},
	path::Path,
};

struct Logger;
impl log::Log for Logger {
	fn log(&self, record: &log::Record) {
		/*
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
		*/

		if let Ok(mut f) = std::fs::OpenOptions::new()
			.append(true)
			.create(true)
			.truncate(false)
			.open("gm_rekinect.log")
		{
			use std::io::Write;
			let _ = if record.level() != log::Level::Info {
				writeln!(f, "gm_rekinect: [{}] {}", record.level(), record.args())
			} else {
				writeln!(f, "gm_rekinect: {}", record.args())
			};
		}
	}

	#[inline]
	fn enabled(&self, metadata: &log::Metadata) -> bool {
		metadata.level() <= log::Level::Info
	}

	fn flush(&self) {}
}

#[repr(u8)]
#[derive(Debug, Clone, Copy)]
enum GmodLuaInterfaceRealm {
	Client = 0,
	Server = 1,
	Menu = 2,
}

type CreateLuaInterfaceFn = extern "cdecl" fn(this: *mut c_void, GmodLuaInterfaceRealm, bool) -> *mut c_void;
type LuaInterfaceInitFn = extern "cdecl" fn(this: *mut c_void, *mut c_void, bool);
type LuaInterfaceShutdownFn = extern "cdecl" fn(this: *mut c_void);

#[link(name = "gm_rekinect_glua", kind = "static")]
extern "C" {
	fn get_lua_shared(create_interface_fn: *const ()) -> *mut c_void;
	fn open_lua_state(i_lua_shared: *mut c_void, realm: GmodLuaInterfaceRealm) -> *mut c_void;
	fn get_lua_state(c_lua_interface: *mut c_void) -> *mut c_void;
	fn lua_state_realm(i_lua_interface: *mut c_void) -> GmodLuaInterfaceRealm;
	fn lookup_vtable(vtable: *mut c_void, index: usize) -> *mut c_void;
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

	unsafe {
		log::info!("gm_rekinect loaded!");

		lua.get_global(lua_string!("motionsensor"));
		log::info!("1");
		if lua.is_nil(-1) {
			lua.create_table(0, 0);
			lua.set_global(lua_string!("motionsensor"));
			lua.get_global(lua_string!("motionsensor"));
		}

		log::info!("2");
		lua.push_string("Start");
		lua.push_function(start);
		lua.set_table(-3);

		log::info!("3");
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

		log::info!("4");
		lua.push_string("GetColourMaterial");
		lua.push_function(get_colour_material);
		lua.set_table(-3);

		lua.pop();
		log::info!("5");

		lua.get_global(lua_string!("MENU_DLL"));
		lua.get_global(lua_string!("FindMetaTable"));
		lua.push_string("Player");
		lua.call(1, 1);

		if !lua.is_nil(-1) {
			log::info!("6");
			lua.push_string("MotionSensorPos");
			lua.push_function(player_motion_sensor_pos);
			lua.set_table(-3);
		}

		log::info!("7");
		lua.pop();

		log::info!("ayyyyyyyyyy");
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
		log::info!("bro!!!");
	}
}

#[gmod13_close]
fn gmod13_close(_lua: gmod::lua::State) {
	// FIXME refcount this
	unsafe { KINECT = None };
}

static mut CREATE_LUA_INTERFACE: Option<gmod::detour::RawDetour> = None;
static mut LUA_INTERFACE_INIT: Option<gmod::detour::RawDetour> = None;

// Support for DLL injecting
#[ctor::ctor]
fn ctor() {
	set_panic_handler();

	std::fs::remove_file("gm_rekinect.log").ok();

	log::set_logger(&Logger).ok();
	log::set_max_level(log::LevelFilter::Info);

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

		let i_lua_shared = get_lua_shared(
			*lib.get::<*const ()>(b"CreateInterface")
				.expect("Failed to find CreateInterface in lua_shared"),
		);
		if i_lua_shared.is_null() {
			panic!("Failed to get ILuaShared");
		}

		{
			let cl = open_lua_state(i_lua_shared, GmodLuaInterfaceRealm::Client);
			let sv = open_lua_state(i_lua_shared, GmodLuaInterfaceRealm::Server);

			// This detection really sucks, can't really think of anything better
			if cl.is_null() && sv.is_null() {
				// We're being injected if the client and server Lua states are inactive
			} else {
				// We're being loaded by GMOD_LoadBinaryModule
				return;
			}
		}

		unsafe extern "cdecl" fn lua_interface_init(this: *mut c_void, a: *mut c_void, b: bool) {
			log::info!("ILuaInterface::Init");

			let trampoline = core::mem::transmute::<_, LuaInterfaceInitFn>(LUA_INTERFACE_INIT.as_ref().unwrap().trampoline() as *const ());
			trampoline(this, a, b);

			let lua = get_lua_state(this);
			log::info!("ILuaInterface::Init -> {:?}", lua);
			if !lua.is_null() {
				gmod13_open(gmod::lua::State(lua));
				log::info!("Injected :)");
			}
		}

		unsafe extern "cdecl" fn create_lua_interface(this: *mut c_void, realm: GmodLuaInterfaceRealm, a: bool) -> *mut c_void {
			log::info!("ILuaShared::CreateLuaInterface({:?}, {:?}, {})", this, realm, a);

			let trampoline = core::mem::transmute::<_, CreateLuaInterfaceFn>(CREATE_LUA_INTERFACE.as_ref().unwrap().trampoline() as *const ());
			let i_lua_interface = trampoline(this, realm, a);
			log::info!("ILuaShared::CreateLuaInterface -> {:?}", i_lua_interface);

			if !i_lua_interface.is_null() {
				let vtable_create_lua_interface = lookup_vtable(i_lua_interface, 1);

				log::info!("ILuaInterface::Init = {:?}", vtable_create_lua_interface);

				LUA_INTERFACE_INIT = Some({
					let lua_interface_init = gmod::detour::RawDetour::new(vtable_create_lua_interface as *const (), lua_interface_init as *const ())
						.expect("Failed to hook ILuaShared::CreateLuaInterface");
					lua_interface_init.enable().expect("Failed to enable ILuaShared::CreateLuaInterface hook");
					lua_interface_init
				});
			}

			i_lua_interface
		}

		log::info!("Hooking ILuaShared::CreateLuaInterface");

		CREATE_LUA_INTERFACE = Some({
			let vtable_create_lua_interface = lookup_vtable(i_lua_shared, 4);
			let create_lua_interface = gmod::detour::RawDetour::new(vtable_create_lua_interface as *const (), create_lua_interface as *const ())
				.expect("Failed to hook ILuaShared::CreateLuaInterface");
			create_lua_interface
				.enable()
				.expect("Failed to enable ILuaShared::CreateLuaInterface hook");
			create_lua_interface
		});

		std::mem::forget(lib);
	}
}

#[ctor::dtor]
fn dtor() {
	unsafe {
		if let Some(create_lua_interface) = CREATE_LUA_INTERFACE.take() {
			create_lua_interface.disable().ok();
		}

		if let Some(lua_interface_init) = LUA_INTERFACE_INIT.take() {
			lua_interface_init.disable().ok();
		}

		KINECT = None;
	}
}

fn set_panic_handler() {
	std::panic::set_hook(Box::new(move |panic| {
		std::fs::write(
			format!("gm_rekinect_panic_{}.log", std::thread::current().id().as_u64()),
			format!("{:#?}", panic),
		)
		.ok();
	}));
}
