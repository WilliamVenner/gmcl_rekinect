use kinect::*;

static mut INIT_REFCOUNT: usize = 0;
static mut KINECT: Option<KinectState> = None;

pub struct KinectState {
	inner: Kinect,
	pub active: bool,
	pub skeleton: KinectSkeleton,
}
impl KinectState {
	fn new() -> Result<Self, std::io::Error> {
		Ok(Self {
			inner: Kinect::new()?,
			active: false,
			skeleton: KinectSkeleton::default(),
		})
	}

	pub fn update(&mut self) {
		if !self.active {
			return;
		}

		let Some(update) = self.inner.poll() else {
			return;
		};

		self.skeleton = update;
	}

	#[inline]
	pub fn available(&self) -> bool {
		self.inner.available()
	}
}

#[lua_function]
unsafe fn poll(_lua: gmod::lua::State) {
	if let Some(kinect) = &mut KINECT {
		kinect.update();
	}
}

pub unsafe fn init(lua: gmod::lua::State) {
	INIT_REFCOUNT += 1;

	if INIT_REFCOUNT != 1 {
		return;
	}

	match KinectState::new() {
		Ok(kinect) => unsafe {
			KINECT = Some(kinect);

			lua.get_global(lua_string!("hook"));
			lua.get_field(-1, lua_string!("Add"));
			lua.push_string("Think");
			lua.push_string("gmcl_rekinect");
			lua.push_function(poll);
			lua.call(3, 0);
			lua.pop();
		},

		Err(err) => {
			log::error!("{err:?}");
		}
	}
}

pub unsafe fn shutdown() {
	INIT_REFCOUNT = INIT_REFCOUNT.saturating_sub(1);

	if INIT_REFCOUNT == 0 {
		KINECT = None;
	}
}

pub unsafe fn already_initialized() -> bool {
	INIT_REFCOUNT != 0
}

#[inline]
pub unsafe fn state() -> Option<&'static mut KinectState> {
	KINECT.as_mut()
}
