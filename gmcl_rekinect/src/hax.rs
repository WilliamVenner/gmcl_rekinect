use crate::logging;
use fn_abi::abi;
use std::{cell::Cell, ffi::c_void, path::Path};

#[repr(u8)]
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub enum GmodLuaInterfaceRealm {
	Client = 0,
	Server = 1,
	Menu = 2,
}

#[link(name = "gmcl_rekinect_cpp", kind = "static")]
extern "C" {
	pub fn get_lua_shared(create_interface_fn: *const ()) -> *mut c_void;
	pub fn open_lua_interface(i_lua_shared: *mut c_void, realm: GmodLuaInterfaceRealm) -> *mut c_void;
	pub fn get_lua_state(c_lua_interface: *mut c_void) -> *mut c_void;
}

#[derive(Clone, Copy, Debug)]
enum RekinectLuaState {
	Uninitialized,
	InjectedDll,
	BinaryModule(gmod::lua::State),
}

thread_local! {
	static LUA_STATE: Cell<RekinectLuaState> = Cell::new(RekinectLuaState::Uninitialized);
}

pub fn lua_state() -> Option<gmod::lua::State> {
	if let RekinectLuaState::BinaryModule(state) = LUA_STATE.get() {
		Some(state)
	} else {
		None
	}
}

macro_rules! dll_paths {
	($($func:ident => $bin:literal / $linux_main_branch:literal),*) => {
		$(pub fn $func() -> &'static str {
			match () {
				_ if cfg!(all(windows, target_pointer_width = "64")) => concat!("bin/win64/", $bin, ".dll"),
				_ if cfg!(all(target_os = "linux", target_pointer_width = "64")) => concat!("bin/linux64/", $bin, ".so"),

				_ if cfg!(all(target_os = "macos")) => concat!("GarrysMod_Signed.app/Contents/MacOS/", $bin, ".dylib"),

				_ if cfg!(all(windows, target_pointer_width = "32")) => {
					let x86_64_branch = concat!("bin/", $bin, ".dll");
					if Path::new(x86_64_branch).exists() {
						x86_64_branch
					} else {
						concat!("garrysmod/bin/", $bin, ".dll")
					}
				},

				_ if cfg!(all(target_os = "linux", target_pointer_width = "32")) => {
					let x86_64_branch = concat!("bin/linux32/", $bin, ".so");
					if Path::new(x86_64_branch).exists() {
						x86_64_branch
					} else {
						concat!("garrysmod/bin/", $linux_main_branch, ".so")
					}
				},

				_ => panic!("Unsupported platform"),
			}
		})*
	};
}
dll_paths! {
	client_dll_path => "client"/"client",
	lua_shared_dll_path => "lua_shared"/"lua_shared",
	lua_shared_srv_dll_path => "lua_shared"/"lua_shared_srv"
}

#[cfg_attr(target_pointer_width = "64", abi("fastcall"))]
#[cfg_attr(all(target_os = "windows", target_pointer_width = "32"), abi("thiscall"))]
#[cfg_attr(all(target_os = "linux", target_pointer_width = "32"), abi("C"))]
type CLuaManagerStartup = extern "C" fn(this: *mut c_void);

macro_rules! cluamanager_detours {
	($($func:ident => { hook($this_var:ident): $hook:block, $trampoline_var:ident: $trampoline:ident, sigs: $sigfunc:ident => { $($cfg:expr => $sig:literal),* } }),*) => {
		$(
			static mut $trampoline: Option<gmod::detour::RawDetour> = None;

			#[cfg_attr(target_pointer_width = "64", abi("fastcall"))]
			#[cfg_attr(all(target_os = "windows", target_pointer_width = "32"), abi("thiscall"))]
			#[cfg_attr(all(target_os = "linux", target_pointer_width = "32"), abi("C"))]
			unsafe extern "C" fn $func($this_var: *mut c_void) {
				let $trampoline_var = core::mem::transmute::<_, CLuaManagerStartup>($trampoline.as_ref().unwrap().trampoline() as *const ());
				$hook;
			}

			fn $sigfunc() -> gmod::sigscan::Signature {
				match () {
					$(_ if $cfg => gmod::sigscan::signature!($sig),)*
					_ => todo!("Unsupported platform")
				}
			}
		)*
	};
}
cluamanager_detours! {
	client_cluamanager_startup => {
		hook(this): {
			trampoline(this);
			cluamanager_startup();
		},
		trampoline: CLIENT_CLUAMANAGER_STARTUP,
		sigs: client_cluamanager_startup_sig => {
			// string search: "Clientside Lua startup!"
			cfg!(all(target_pointer_width = "64", target_os = "windows")) => "48 89 5C 24 ? 48 89 74 24 ? 57 48 81 EC ? ? ? ? 48 8B 05 ? ? ? ? 48 33 C4 48 89 84 24 ? ? ? ? 48 8B F1 48 8D 0D ? ? ? ? FF 15 ? ? ? ? E8 ? ? ? ?",
			cfg!(all(target_pointer_width = "32", target_os = "windows")) => "55 8B EC 81 EC ? ? ? ? 53 68 ? ? ? ? 8B D9 FF 15 ? ? ? ? 83 C4 04 E8 ? ? ? ? D9 05 ? ? ? ? 68 ? ? ? ?"
		}
	}
}

unsafe fn cluamanager_startup() {
	let lib_path = lua_shared_dll_path();

	let lib = {
		#[cfg(windows)]
		{
			libloading::os::windows::Library::open_already_loaded(lib_path)
		}
		#[cfg(unix)]
		{
			libloading::os::unix::Library::open(Some(lib_path), libc::RTLD_NOLOAD)
				.or_else(|_| libloading::os::unix::Library::open(Some(lib_path), libc::RTLD_NOLOAD))
		}
	}
	.expect("Failed to load lua_shared");

	let i_lua_shared = get_lua_shared(
		*lib.get::<*const ()>(b"CreateInterface")
			.expect("Failed to find CreateInterface in lua_shared"),
	);

	if i_lua_shared.is_null() {
		panic!("Failed to get ILuaShared");
	}

	let c_lua_interface = open_lua_interface(i_lua_shared, GmodLuaInterfaceRealm::Client);
	if c_lua_interface.is_null() {
		panic!("Failed to get CLuaInterface");
	}

	let lua_state = get_lua_state(c_lua_interface);

	{
		static mut GMOD_RS_SET_LUA_STATE: bool = false;
		if !core::mem::replace(&mut GMOD_RS_SET_LUA_STATE, true) {
			gmod::set_lua_state(lua_state);
		}
	}

	crate::init(gmod::lua::State(lua_state));
}

pub unsafe fn init() {
	if is_ctor_binary_module() {
		// If we were loaded by GMOD_LoadBinaryModule, we don't need to hook CLuaManager::Startup
		return;
	}

	LUA_STATE.set(RekinectLuaState::InjectedDll);

	logging::init_for_injected_dll();

	log::info!("DLL injected");

	let client_dll_path = client_dll_path();

	let (dll_path, sig, global, detour) = (
		client_dll_path,
		client_cluamanager_startup_sig(),
		&mut CLIENT_CLUAMANAGER_STARTUP,
		client_cluamanager_startup as *const (),
	);
	log::info!("Hooking CLuaManager::Startup in {dll_path}");

	let cluamanager_startup = sig.scan_module(dll_path).expect("Failed to find CLuaManager::Startup") as *const ();

	*global = Some({
		let cluamanager_startup = gmod::detour::RawDetour::new(cluamanager_startup, detour).expect("Failed to hook CLuaManager::Startup");
		cluamanager_startup.enable().expect("Failed to enable CLuaManager::Startup hook");
		cluamanager_startup
	});
}

unsafe fn is_ctor_binary_module() -> bool {
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

	let cl = open_lua_interface(i_lua_shared, GmodLuaInterfaceRealm::Client);
	let sv = open_lua_interface(i_lua_shared, GmodLuaInterfaceRealm::Server);

	// This detection really sucks, can't really think of anything better
	if cl.is_null() && sv.is_null() {
		// We're being injected if the client and server Lua states are inactive
		false
	} else {
		// We're being loaded by GMOD_LoadBinaryModule
		true
	}
}

pub fn binary_module_init(lua: gmod::lua::State) {
	if !matches!(LUA_STATE.get(), RekinectLuaState::InjectedDll) {
		LUA_STATE.set(RekinectLuaState::BinaryModule(lua));
	}
}
