use std::{path::PathBuf, time::Duration};

#[cfg(windows)]
mod windows;

struct Gmod<P> {
	process: P,
	gmcl_rekinect: PathBuf,
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
				eprintln!("Failed to inject gmcl_rekinect: {err:?}");
				std::thread::sleep(Duration::from_secs(5));
				continue;
			}
		};

		println!("Injected successfully!");

		println!("Waiting for Garry's Mod to close...");

		gmod.wait();
	}
}
