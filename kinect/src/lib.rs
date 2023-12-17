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
	pub fn from_named_bones(bones: KinectSkeletonBones) -> Self {
		Self { bones }
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
		unsafe fn load_rekinect_dyn(backend: &str) -> Option<Kinect> {
			// This function looks kind of weird because we need to be careful about the drop order of the libloading::Library.
			// If we drop the library too early, we might crash in the logging calls when we try to print the error that it returned
			// (as the error is part of that library's address space, which would be deallocated when we drop the library)

			log::info!("{}: Loading...", backend);

			type GmKinectDynInit = unsafe extern "Rust" fn(&'static dyn log::Log) -> Result<Box<dyn KinectBackend>, std::io::Error>;

			let lib = libloading::Library::new(backend);
			let lib = lib.and_then(|lib| Ok((*lib.get::<GmKinectDynInit>(b"gmcl_rekinect_init")?, lib)));

			match lib {
				Ok((init, lib)) => match init(log::logger()) {
					Ok(kinect) => {
						log::info!("{}: OK!", backend);
						Some(Kinect { _lib: lib, backend: kinect })
					}

					Err(err) => {
						log::warn!("{}: {err:?}", backend);
						None
					}
				},

				Err(err) => {
					log::warn!("{}: {err:?}", backend);
					None
				}
			}
		}

		macro_rules! try_load_backend {
			($backend:literal) => {
				for backend in [concat!("garrysmod/lua/bin/", $backend), $backend] {
					if let Some(backend) = unsafe { load_rekinect_dyn(backend) } {
						return Ok(backend);
					}
				}
			};
		}

		if cfg!(windows) {
			try_load_backend!("rekinect_winsdk_v2.dll");
			try_load_backend!("rekinect_winsdk_v1.dll");
		}

		Err(std::io::Error::new(
			std::io::ErrorKind::Unsupported,
			"No backend available, did you remember to install one? https://github.com/WilliamVenner/gmcl_rekinect",
		))
	}

	#[inline]
	pub fn poll(&mut self) -> Option<KinectSkeleton> {
		self.backend.poll()
	}
}
