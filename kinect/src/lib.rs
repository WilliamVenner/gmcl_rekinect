pub const SKELETON_BONE_COUNT: usize = 20;

pub type KinectSkeletonRawBones = [[f32; 3]; SKELETON_BONE_COUNT];

pub trait KinectBackend {
	fn poll(&mut self) -> Option<KinectSkeleton>;
}

#[derive(Clone, Copy, Debug)]
#[allow(clippy::large_enum_variant)]
pub enum KinectSkeleton {
	Untracked,
	Tracked(KinectTrackedSkeleton),
}

#[derive(Clone, Copy)]
pub union KinectTrackedSkeleton {
	raw_bones: KinectSkeletonRawBones,
	bones: KinectSkeletonBones,
}
impl KinectTrackedSkeleton {
	#[inline(always)]
	pub fn from_raw_bones(raw_bones: KinectSkeletonRawBones) -> Self {
		Self { raw_bones }
	}

	#[inline(always)]
	pub fn raw_bones(&self) -> &KinectSkeletonRawBones {
		unsafe { &self.raw_bones }
	}

	#[inline(always)]
	pub fn bones(&self) -> &KinectSkeletonBones {
		unsafe { &self.bones }
	}
}
impl std::fmt::Debug for KinectTrackedSkeleton {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		self.bones().fmt(f)
	}
}
impl Default for KinectTrackedSkeleton {
	fn default() -> Self {
		Self {
			raw_bones: [[0.0; 3]; SKELETON_BONE_COUNT],
		}
	}
}

#[derive(Clone, Copy, Debug)]
pub struct KinectSkeletonBones {
	pub hip_center: [f32; 3],
	pub spine: [f32; 3],
	pub shoulder_center: [f32; 3],
	pub head: [f32; 3],
	pub shoulder_left: [f32; 3],
	pub elbow_left: [f32; 3],
	pub wrist_left: [f32; 3],
	pub hand_left: [f32; 3],
	pub shoulder_right: [f32; 3],
	pub elbow_right: [f32; 3],
	pub wrist_right: [f32; 3],
	pub hand_right: [f32; 3],
	pub hip_left: [f32; 3],
	pub knee_left: [f32; 3],
	pub ankle_left: [f32; 3],
	pub foot_left: [f32; 3],
	pub hip_right: [f32; 3],
	pub knee_right: [f32; 3],
	pub ankle_right: [f32; 3],
	pub foot_right: [f32; 3],
}

pub struct Kinect {
	backend: Box<dyn KinectBackend>,
	_lib: libloading::Library,
}
impl Kinect {
	pub fn new() -> Result<Self, std::io::Error> {
		unsafe fn load_rekinect_dyn(lib: &str) -> Result<Kinect, std::io::Error> {
			type GmKinectDynInit = unsafe extern "Rust" fn() -> Result<Box<dyn KinectBackend>, std::io::Error>;

			let lib = libloading::Library::new(lib).map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, err))?;

			let init = lib
				.get::<GmKinectDynInit>(b"gm_rekinect_init")
				.map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, err))?;

			init().map(|backend| Kinect { _lib: lib, backend })
		}

		macro_rules! try_load_backend {
			($backend:literal) => {
				match { unsafe { load_rekinect_dyn(concat!("garrysmod/bin/", $backend)) }.or_else(|_| unsafe { load_rekinect_dyn($backend) }) } {
					Err(err) => log::warn!("{}: {err:?}", $backend),
					backend @ Ok(_) => {
						log::info!("{}: OK!", $backend);
						return backend;
					}
				}
			};
		}

		if cfg!(windows) {
			try_load_backend!("gm_rekinect_winsdk_v1.dll");
			try_load_backend!("gm_rekinect_libfreenect.dll");
		} else if cfg!(target_os = "linux") {
			try_load_backend!("libgm_rekinect_libfreenect.so");
		} else if cfg!(target_os = "macos") {
			try_load_backend!("libgm_rekinect_libfreenect.dylib");
		}

		Err(std::io::Error::new(
			std::io::ErrorKind::Unsupported,
			"No backend available, did you remember to install one? https://github.com/WilliamVenner/gm_rekinect",
		))
	}

	#[inline]
	pub fn poll(&mut self) -> Option<KinectSkeleton> {
		self.backend.poll()
	}
}
