use crate::hax;
use std::fs::File;

static mut LOGGER: Logger = Logger(None);

struct Logger(Option<File>);
impl log::Log for Logger {
	fn log(&self, record: &log::Record) {
		if let Some(lua) = hax::lua_state() {
			unsafe {
				lua.get_global(lua_string!("print"));
				lua.push_string(&if record.level() != log::Level::Info {
					format!("gmcl_rekinect: [{}] {}", record.level(), record.args())
				} else {
					format!("gmcl_rekinect: {}", record.args())
				});
				lua.call(1, 0);
			}
		} else if let Some(mut f) = self.0.as_ref() {
			use std::io::Write;
			let _ = if record.level() != log::Level::Info {
				writeln!(f, "gmcl_rekinect: [{}] {}", record.level(), record.args())
			} else {
				writeln!(f, "gmcl_rekinect: {}", record.args())
			};
		}
	}

	#[inline]
	fn enabled(&self, metadata: &log::Metadata) -> bool {
		metadata.level() <= log::Level::Info
	}

	fn flush(&self) {}
}

pub unsafe fn init_for_injected_dll() {
	std::fs::remove_file("gmcl_rekinect.log").ok();

	LOGGER = Logger(
		std::fs::OpenOptions::new()
			.append(true)
			.create(true)
			.truncate(false)
			.open("gmcl_rekinect.log")
			.ok(),
	);

	init_for_binary_module();
}

pub unsafe fn init_for_binary_module() {
	log::set_logger(&LOGGER).ok();
	log::set_max_level(log::LevelFilter::Info);
}
