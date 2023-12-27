pub const SKELETON_BONE_COUNT: usize = 20;
pub const EXTENDED_SKELETON_BONE_COUNT: usize = 7;

pub type KinectSkeletonRawBones = [[f32; 3]; SKELETON_BONE_COUNT];
pub type KinectExtendedSkeletonRawBones = [[f32; 3]; EXTENDED_SKELETON_BONE_COUNT];

pub trait KinectBackend {
	fn poll(&mut self) -> Option<KinectSkeleton>;
	fn available(&self) -> bool;
}

#[derive(Clone, Copy, Debug, Default)]
#[allow(clippy::large_enum_variant)]
pub enum KinectSkeleton {
	#[default]
	Untracked,
	Tracked(KinectTrackedSkeleton),
	TrackedExtended(KinectTrackedSkeleton, KinectTrackedExtendedSkeleton),
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

#[derive(Clone, Copy)]
pub union KinectTrackedExtendedSkeleton {
	raw_bones: KinectExtendedSkeletonRawBones,
	bones: KinectExtendedSkeletonBones,
}
impl KinectTrackedExtendedSkeleton {
	#[inline(always)]
	pub fn from_raw_bones(raw_bones: KinectExtendedSkeletonRawBones) -> Self {
		Self { raw_bones }
	}

	#[inline(always)]
	pub fn from_named_bones(bones: KinectExtendedSkeletonBones) -> Self {
		Self { bones }
	}

	#[inline(always)]
	pub fn raw_bones(&self) -> &KinectExtendedSkeletonRawBones {
		unsafe { &self.raw_bones }
	}

	#[inline(always)]
	pub fn bones(&self) -> &KinectExtendedSkeletonBones {
		unsafe { &self.bones }
	}
}
impl std::fmt::Debug for KinectTrackedExtendedSkeleton {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		self.bones().fmt(f)
	}
}
impl Default for KinectTrackedExtendedSkeleton {
	fn default() -> Self {
		Self {
			raw_bones: [[0.0; 3]; EXTENDED_SKELETON_BONE_COUNT],
		}
	}
}

#[derive(Clone, Copy, Debug)]
pub struct KinectExtendedSkeletonBones {
	pub spine_base: [f32; 3],
	pub neck: [f32; 3],
	pub spine_shoulder: [f32; 3],
	pub hand_tip_left: [f32; 3],
	pub thumb_left: [f32; 3],
	pub hand_tip_right: [f32; 3],
	pub thumb_right: [f32; 3],
}

pub struct DynKinectBackend {
	backend: Box<dyn KinectBackend>,
	_lib: libloading::Library,
}
impl DynKinectBackend {
	unsafe fn load(backend: &str) -> Option<Self> {
		log::info!("{}: Loading...", backend);

		type GmKinectDynInit = unsafe extern "Rust" fn(&'static dyn log::Log) -> Result<Box<dyn KinectBackend>, std::io::Error>;

		let lib = libloading::Library::new(backend);
		let lib = lib.and_then(|lib| Ok((*lib.get::<GmKinectDynInit>(b"gmcl_rekinect_init")?, lib)));

		match lib {
			Ok((init, lib)) => match init(log::logger()) {
				Ok(kinect) => {
					log::info!("{}: OK!", backend);
					Some(Self { _lib: lib, backend: kinect })
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
}

pub struct Kinect {
	backends: Box<[DynKinectBackend]>,
}
impl Kinect {
	pub fn new() -> Result<Self, std::io::Error> {
		let mut backends = Vec::new();

		macro_rules! try_load_backend {
			($backend:expr) => {
				for backend in [concat!("garrysmod/lua/bin/", $backend), $backend] {
					if let Some(backend) = unsafe { DynKinectBackend::load(backend) } {
						backends.push(backend);
						break;
					}
				}
			};
		}

		if cfg!(all(windows, target_pointer_width = "64")) {
			try_load_backend!("rekinect_winsdk_v2_win64.dll");
			try_load_backend!("rekinect_winsdk_v1_win64.dll");
		} else if cfg!(all(windows, target_pointer_width = "32")) {
			try_load_backend!("rekinect_winsdk_v2_win32.dll");
			try_load_backend!("rekinect_winsdk_v1_win32.dll");
		}

		if !backends.is_empty() {
			Ok(Kinect {
				backends: backends.into_boxed_slice(),
			})
		} else {
			Err(std::io::Error::new(
				std::io::ErrorKind::Unsupported,
				"No backend available, did you remember to install one? https://github.com/WilliamVenner/gmcl_rekinect",
			))
		}
	}

	#[inline]
	pub fn poll(&mut self) -> Option<KinectSkeleton> {
		self.backends.iter_mut().find_map(|backend| backend.backend.poll())
	}

	#[inline]
	pub fn available(&self) -> bool {
		self.backends.iter().any(|backend| backend.backend.available())
	}
}
