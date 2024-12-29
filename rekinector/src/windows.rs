use super::Gmod;
use crate::InjectedGmod;
use dll_syringe::process::Process;
use eyre::OptionExt;
use std::{
	ffi::OsStr,
	mem::size_of,
	os::windows::io::AsRawHandle,
	path::{Path, PathBuf},
	time::Duration,
};
use windows::{
	Wdk::System::Threading::{NtQueryInformationProcess, ProcessBasicInformation},
	Win32::{
		Foundation::{BOOL, HANDLE, HMODULE, WAIT_FAILED},
		System::{
			ProcessStatus::GetModuleFileNameExA,
			SystemInformation::{GetNativeSystemInfo, PROCESSOR_ARCHITECTURE_INTEL, SYSTEM_INFO},
			Threading::{
				IsWow64Process, OpenProcess, WaitForSingleObject, INFINITE, PROCESS_ACCESS_RIGHTS, PROCESS_BASIC_INFORMATION,
				PROCESS_QUERY_LIMITED_INFORMATION,
			},
		},
	},
};

const MAX_PATH: usize = 32767;

impl Gmod<dll_syringe::process::OwnedProcess> {
	pub fn find() -> Option<Self> {
		dll_syringe::process::OwnedProcess::find_all_by_name("gmod.exe")
			.into_iter()
			.filter_map(|process| {
				let handle = HANDLE(process.as_raw_handle() as isize);

				let exe_path = get_process_exe_path(handle).ok()?;

				if exe_path.extension() != Some(OsStr::new("exe")) {
					return None;
				}

				let Some(exe) = exe_path.file_name() else {
					return None;
				};

				let Some(mut exe_path) = exe_path.parent() else {
					return None;
				};

				if exe_path.file_name() == Some(OsStr::new("win64")) {
					exe_path = match exe_path.parent().and_then(|p| p.parent()) {
						Some(p) => p,
						None => return None,
					};
				}

				let Ok(is_x86) = is_x86_process(handle) else {
					return None;
				};

				// Check that this isn't a subprocess
				let Ok(false) = is_subprocess(handle, exe) else {
					return None;
				};

				let mut garrysmod_path = exe_path.join("garrysmod");

				if garrysmod_path.is_dir() {
					return Some(Gmod {
						process,
						gmod_dir: exe_path.to_owned(),
						gmcl_rekinect: {
							garrysmod_path.push("lua");
							garrysmod_path.push("bin");
							garrysmod_path.push(format!("gmcl_rekinect_win{}.dll", if is_x86 { "32" } else { "64" }));
							garrysmod_path
						},
					});
				}

				None
			})
			.next()
	}

	pub fn pid(&self) -> Option<u32> {
		self.process.pid().ok().map(|pid| pid.get())
	}

	pub fn inject(&self) -> eyre::Result<InjectedGmod<dll_syringe::process::OwnedProcessModule>> {
		if !self.gmcl_rekinect.is_file() {
			return Err(eyre::eyre!(
				"You forgot to install the gmcl_rekinect binary module, please read the installation instructions."
			));
		}

		println!("Waiting for Lua initialization...");

		while self.process.find_module_by_name("lua_shared")?.is_none() {
			std::thread::sleep(Duration::from_secs(1));
		}

		Ok(InjectedGmod {
			process: dll_syringe::Syringe::for_process(self.process.try_clone()?)
				.find_or_inject(&self.gmcl_rekinect)?
				.try_to_owned()?,
		})
	}
}

impl InjectedGmod<dll_syringe::process::OwnedProcessModule> {
	pub fn wait(self) {
		let sync_res: eyre::Result<()> = (|| unsafe {
			const SYNCHRONIZE: PROCESS_ACCESS_RIGHTS = PROCESS_ACCESS_RIGHTS(0x00100000);

			let sync = OpenProcess(SYNCHRONIZE, BOOL::from(false), self.process.process().pid()?.get() as _)?;

			if WaitForSingleObject(sync, INFINITE) == WAIT_FAILED {
				return Err(eyre::eyre!(std::io::Error::last_os_error()));
			}

			Ok(())
		})();

		drop(self);

		if let Err(err) = sync_res {
			eprintln!("Failed to wait for Gmod to close: {err:?}");
			println!("Press ENTER to continue...");
			std::io::stdin().read_line(&mut String::new()).ok();
		}
	}
}

fn is_x86_process(process: HANDLE) -> eyre::Result<bool> {
	unsafe {
		let mut system_info: SYSTEM_INFO = core::mem::zeroed();
		GetNativeSystemInfo(&mut system_info);

		if system_info.Anonymous.Anonymous.wProcessorArchitecture == PROCESSOR_ARCHITECTURE_INTEL {
			// This computer is 32-bit
			return Ok(true);
		}

		let mut is_wow_64 = BOOL(0);
		IsWow64Process(process, &mut is_wow_64)?;
		Ok(is_wow_64 == BOOL(1))
	}
}

fn is_subprocess(process: HANDLE, process_name: &OsStr) -> eyre::Result<bool> {
	Ok(unsafe {
		let mut info: PROCESS_BASIC_INFORMATION = core::mem::zeroed();
		NtQueryInformationProcess(
			process,
			ProcessBasicInformation,
			&mut info as *mut _ as *mut _,
			size_of::<PROCESS_BASIC_INFORMATION>() as _,
			core::ptr::null_mut(),
		)
		.ok()?;

		if info.InheritedFromUniqueProcessId == 0 {
			return Ok(false);
		}

		let parent = OpenProcess(
			PROCESS_QUERY_LIMITED_INFORMATION,
			BOOL::from(false),
			info.InheritedFromUniqueProcessId as _,
		)?;

		let exe_path = get_process_exe_path(parent)?;
		let exe = exe_path.file_name().ok_or_eyre("Failed to get parent executable name")?;

		exe == process_name
	})
}

fn get_process_exe_path(process: HANDLE) -> eyre::Result<PathBuf> {
	unsafe {
		let mut exe_path = [0u8; MAX_PATH];
		let len = GetModuleFileNameExA(process, HMODULE(0), &mut exe_path);
		if len == 0 {
			return Err(eyre::eyre!(std::io::Error::last_os_error()));
		}
		let exe_path = &exe_path[..len as usize];
		let exe_path = OsStr::from_encoded_bytes_unchecked(exe_path);
		let exe_path = Path::new(exe_path);
		Ok(exe_path.to_owned())
	}
}
