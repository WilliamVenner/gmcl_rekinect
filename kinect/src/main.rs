use std::sync::atomic::AtomicBool;

static SHUTDOWN: AtomicBool = AtomicBool::new(false);

fn main() -> Result<(), std::io::Error> {
	if std::env::var_os("RUST_LOG").is_none() {
		std::env::set_var("RUST_LOG", "info");
	}

	env_logger::init();
	log::set_max_level(log::LevelFilter::Info);

	ctrlc::set_handler(move || {
		if SHUTDOWN.swap(true, std::sync::atomic::Ordering::AcqRel) {
			println!("Aborting process");
			std::process::exit(1);
		} else {
			println!("Trying to shut down gracefully... press CTRL+C again to abort");
		}
	})
	.ok();

	let exe = std::env::current_exe().ok();
	if let Some(exe) = exe.as_ref().and_then(|exe| exe.parent()) {
		if cfg!(all(windows, target_pointer_width = "64")) {
			std::fs::copy(exe.join("rekinect_winsdk_v2.dll"), exe.join("rekinect_winsdk_v2_win64.dll")).unwrap();
			std::fs::copy(exe.join("rekinect_winsdk_v1.dll"), exe.join("rekinect_winsdk_v1_win64.dll")).unwrap();
		} else if cfg!(all(windows, target_pointer_width = "32")) {
			std::fs::copy(exe.join("rekinect_winsdk_v2.dll"), exe.join("rekinect_winsdk_v2_win32.dll")).unwrap();
			std::fs::copy(exe.join("rekinect_winsdk_v1.dll"), exe.join("rekinect_winsdk_v1_win32.dll")).unwrap();
		}
	}

	{
		let mut kinect = kinect::Kinect::new().unwrap();
		while !SHUTDOWN.load(std::sync::atomic::Ordering::Acquire) {
			if let Some(update) = kinect.poll() {
				println!("{:#?}", update);
			}
			std::thread::sleep(std::time::Duration::from_millis(50));
		}
	}

	println!("Shut down gracefully");

	Ok(())
}
