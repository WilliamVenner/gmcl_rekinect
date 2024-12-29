use std::{
	ffi::OsStr,
	path::PathBuf,
	time::{Duration, SystemTime},
};

#[cfg(windows)]
mod windows;

struct Gmod<P> {
	process: P,
	gmcl_rekinect: PathBuf,
	gmod_dir: PathBuf,
}

struct InjectedGmod<P> {
	process: P,
}

fn main() {
	println!(concat!("rekinector v", env!("CARGO_PKG_VERSION"), " by Billy"));

	loop {
		println!("Waiting for Garry's Mod to start...");

		let gmod = loop {
			match Gmod::find() {
				Some(gmod) => break gmod,
				None => {
					std::thread::sleep(Duration::from_secs(5));
					continue;
				}
			}
		};

		match gmod.pid() {
			Some(pid) => println!("Found Garry's Mod (pid {pid})"),
			None => println!("Found Garry's Mod (pid unknown)"),
		}

		println!("Injecting gmcl_rekinect...");

		let gmod = match gmod.inject() {
			Ok(gmod) => gmod,
			Err(err) => {
				let now = SystemTime::now();

				eprintln!("Failed to inject gmcl_rekinect: {err:?}");

				// Print the logs and panic if found and recently modified...

				let is_recent_log = |(path, modified): (PathBuf, SystemTime)| {
					if now.duration_since(modified).is_ok_and(|elapsed| elapsed <= Duration::from_secs(10)) {
						Some(path)
					} else {
						None
					}
				};

				let log_path = Some(gmod.gmod_dir.join("gmcl_rekinect.log")).and_then(|path| {
					let metadata = path.metadata().ok()?;
					let modified = metadata.modified().ok()?;

					Some((path, modified))
				});

				let panic_logs = gmod.gmod_dir.read_dir().into_iter().flat_map(|gmod_dir| {
					gmod_dir
						.filter_map(|entry| entry.ok())
						.filter_map(|entry| {
							let path = entry.path();

							let name = path.file_name()?.to_str()?;
							if !name.starts_with("gmcl_rekinect_panic_") || entry.path().extension() != Some(OsStr::new("log")) {
								return None;
							}

							let metadata = entry.metadata().ok()?;
							let modified = metadata.modified().ok()?;

							Some((path, modified))
						})
						.filter_map(is_recent_log)
						.filter_map(|path| std::fs::read_to_string(path).ok())
				});

				if let Some(logs) = log_path.and_then(is_recent_log).and_then(|path| std::fs::read_to_string(path).ok()) {
					println!("\n========= LOGS =========\n{logs}");
				}

				for log in panic_logs {
					println!("{log}\n");
				}

				std::thread::sleep(Duration::from_secs(5));
				continue;
			}
		};

		println!("Injected successfully!");

		println!("Waiting for Garry's Mod to close...");

		gmod.wait();
	}
}
