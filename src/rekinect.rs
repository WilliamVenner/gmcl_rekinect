use kinect::*;
use std::{
	ffi::OsString,
	fs::OpenOptions,
	mem::{size_of, ManuallyDrop},
	path::Path,
};

static mut INIT_REFCOUNT: usize = 0;
static mut KINECT: Option<KinectState> = None;

const MMAP_FILE_SIZE: u64 =
	(size_of::<u8>() + size_of::<u8>() + size_of::<u8>() + size_of::<u16>() + (size_of::<[f32; 3]>() * kinect::SKELETON_BONE_COUNT)) as u64;

const MMAP_KINECT_SKELETON_NONE: u8 = 0;
const MMAP_KINECT_SKELETON_TRACKED: u8 = 1;

const MMAP_SHUTDOWN: usize = 0;
const MMAP_ACTIVE: usize = 1;
const MMAP_SYNC: std::ops::Range<usize> = 2..4;
const MMAP_SKELETON: usize = 4;
const MMAP_SKELETON_BONES: std::ops::Range<usize> = 5..MMAP_FILE_SIZE as usize;

pub struct KinectState {
	mmap: memmap::MmapMut,
	pub skeleton: Option<[[f32; 3]; kinect::SKELETON_BONE_COUNT]>,
	kind: KinectStateKind,
}
impl KinectState {
	fn new() -> Result<Self, std::io::Error> {
		// We're a client if garrysmod/cache/gmcl_rekinect/klient_pid.dat exists
		let mmap_name = OsString::from(format!("kinect_{}", std::process::id()));
		let client = 'client: {
			if let Ok(dir) = std::fs::read_dir("garrysmod/cache/gmcl_rekinect") {
				for entry in dir.flatten() {
					let entry = entry.path();
					if entry.file_name() == Some(mmap_name.as_os_str()) {
						break 'client true;
					}
				}
			}
			break 'client false;
		};
		if !client {
			// Clean up old mmaps
			std::fs::remove_dir_all("garrysmod/cache/gmcl_rekinect").ok();
		}

		std::fs::create_dir_all("garrysmod/cache/gmcl_rekinect")?;

		let mmap_path = Path::new("garrysmod/cache/gmcl_rekinect").join(mmap_name);

		let f = OpenOptions::new().write(true).read(true).truncate(false).create(true).open(mmap_path)?;

		f.set_len(MMAP_FILE_SIZE)?;

		let mut mmap = unsafe { memmap::MmapMut::map_mut(&f)? };

		if client {
			log::info!("client connected to mmap");

			let mut client = Self {
				mmap,
				skeleton: None,
				kind: KinectStateKind::Client { sync: None },
			};

			client.update();

			Ok(client)
		} else {
			log::info!("mmap server opened");

			let inner = Kinect::new()?;

			mmap.fill(0);
			mmap.flush().ok();

			Ok(Self {
				mmap,
				skeleton: None,
				kind: KinectStateKind::Server {
					inner: ManuallyDrop::new(inner),
					sync: 0,
				},
			})
		}
	}

	pub fn update(&mut self) {
		match &mut self.kind {
			KinectStateKind::Server { inner, sync } => {
				if self.mmap[MMAP_ACTIVE] != 1 {
					return;
				}

				let Some(update) = inner.poll() else {
					return;
				};

				*sync = sync.wrapping_add(1);
				self.mmap[MMAP_SYNC].copy_from_slice(&u16::to_ne_bytes(*sync));

				if let KinectSkeleton::Tracked(pos) = update {
					self.mmap[MMAP_SKELETON] = MMAP_KINECT_SKELETON_TRACKED;

					let skeleton = self.skeleton.get_or_insert_default();

					for ((vec, mmap), skeleton) in pos
						.raw_bones()
						.iter()
						.zip(self.mmap[MMAP_SKELETON_BONES].array_chunks_mut::<{ size_of::<[f32; 3]>() }>())
						.zip(skeleton.iter_mut())
					{
						mmap[0..4].copy_from_slice(&f32::to_ne_bytes(vec[0]));
						mmap[4..8].copy_from_slice(&f32::to_ne_bytes(vec[1]));
						mmap[8..12].copy_from_slice(&f32::to_ne_bytes(vec[2]));

						*skeleton = *vec;
					}

					self.mmap
						.flush_range(MMAP_SYNC.start, (MMAP_SYNC.start..MMAP_SKELETON_BONES.end).len())
						.ok();
				} else {
					self.mmap[MMAP_SKELETON] = MMAP_KINECT_SKELETON_NONE;
					self.mmap.flush_range(MMAP_SYNC.start, (MMAP_SYNC.start..MMAP_SKELETON).len()).ok();

					self.skeleton = None;
				}
			}

			KinectStateKind::Client { sync } => {
				let shutdown = self.mmap[MMAP_SHUTDOWN];
				if shutdown == 1 {
					log::info!("trying to promote to server");

					// Promote to server
					if let Ok(inner) = Kinect::new() {
						if core::mem::replace(&mut self.mmap[MMAP_SHUTDOWN], 0) != 1 {
							return self.update();
						}

						if self.mmap.flush_range(MMAP_SHUTDOWN, 1).is_ok() {
							self.kind = KinectStateKind::Server {
								inner: ManuallyDrop::new(inner),
								sync: sync.unwrap_or(0),
							};

							log::info!("promoted to server");

							return self.update();
						}
					}
					return;
				}

				let new_sync = Some(u16::from_ne_bytes(self.mmap[MMAP_SYNC].try_into().unwrap()));
				if new_sync == core::mem::replace(sync, new_sync) {
					// No changes
					return;
				}

				match self.mmap[MMAP_SKELETON] {
					MMAP_KINECT_SKELETON_NONE => {
						self.skeleton = None;
					}

					MMAP_KINECT_SKELETON_TRACKED => {
						let skeleton = self.skeleton.get_or_insert_default();
						for (bone, skeleton) in self.mmap[MMAP_SKELETON_BONES]
							.array_chunks::<{ size_of::<[f32; 3]>() }>()
							.zip(skeleton.iter_mut())
						{
							*skeleton = [
								f32::from_ne_bytes(bone[0..4].try_into().unwrap()),
								f32::from_ne_bytes(bone[4..8].try_into().unwrap()),
								f32::from_ne_bytes(bone[8..12].try_into().unwrap()),
							];
						}
					}

					_ => unreachable!(),
				}
			}
		}
	}

	pub fn active(&self) -> bool {
		self.mmap[MMAP_ACTIVE] == 1
	}

	pub fn set_active(&mut self, active: bool) {
		self.mmap[MMAP_ACTIVE] = active as u8;
		self.mmap.flush_range(MMAP_ACTIVE, 1).ok();
	}
}
impl Drop for KinectState {
	fn drop(&mut self) {
		if let KinectStateKind::Server { inner, .. } = &mut self.kind {
			// Shut down Kinect
			unsafe { ManuallyDrop::drop(inner) };

			// Mark shutdown byte
			self.mmap[MMAP_SHUTDOWN] = 1;
			self.mmap.flush_range(MMAP_SHUTDOWN, 1).ok();
		}
	}
}

enum KinectStateKind {
	Server { inner: ManuallyDrop<Kinect>, sync: u16 },
	Client { sync: Option<u16> },
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
