#![cfg(windows)]

use kinect::{KinectBackend, KinectSkeleton, KinectSkeletonRawBones, KinectTrackedSkeleton};
use std::{ffi::c_void, marker::PhantomData, mem::ManuallyDrop, os::windows::io::AsRawHandle};
use windows::{
	core::HRESULT,
	Win32::{
		Foundation::{HANDLE, LPARAM, WPARAM},
		System::Threading::GetThreadId,
		UI::WindowsAndMessaging::{PostThreadMessageW, WM_QUIT},
	},
};

const BONE_COUNT: usize = 20;

#[inline]
fn convert_kinect_coordinate_space_to_gmod(vector: &Vector4) -> [f32; 3] {
	[-vector.x, vector.z, vector.y]
}

#[link(name = "kinect_winsdk_v1_cpp", kind = "static")]
extern "C" {
	fn WinSdkKinectV1_Create(callback: CWinSdkKinectV1Callback, userdata: *mut c_void, result: &mut HRESULT) -> *mut c_void;
	fn WinSdkKinectV1_Destroy(ptr: *mut c_void);
	fn WinSdkKinectV1_Run(ptr: *mut c_void) -> HRESULT;
	fn WinSdkKinectV1_Available(ptr: *mut c_void) -> bool;
}

type CWinSdkKinectV1Callback = extern "C" fn(WinSdkKinectV1SkeletonUpdate, *mut c_void);
type WinSdkKinectV1Callback<U> = extern "C" fn(WinSdkKinectV1SkeletonUpdate, &mut U);

struct SendPtr<T>(*mut T);
unsafe impl<T> Send for SendPtr<T> {}
unsafe impl<T> Sync for SendPtr<T> {}

#[repr(C)]
struct WinSdkKinectV1SkeletonUpdate {
	skeleton_index: usize,
	skeleton: *const WinSdkKinectV1Skeleton,
}
impl WinSdkKinectV1SkeletonUpdate {
	#[inline]
	fn skeleton(&self) -> Option<&WinSdkKinectV1Skeleton> {
		if !self.skeleton.is_null() {
			Some(unsafe { &*self.skeleton })
		} else {
			None
		}
	}
}
impl std::fmt::Debug for WinSdkKinectV1SkeletonUpdate {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("WinSdkKinectV1SkeletonUpdate")
			.field("skeleton_index", &self.skeleton_index)
			.field("skeleton", &self.skeleton())
			.finish()
	}
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct Vector4 {
	x: f32,
	y: f32,
	z: f32,
	w: f32,
}

#[derive(Clone, Copy)]
#[repr(C)]
union WinSdkKinectV1Skeleton {
	raw: [Vector4; BONE_COUNT],
	named: NamedSensorBones,
}
impl WinSdkKinectV1Skeleton {
	#[inline(always)]
	fn named_bones(&self) -> &NamedSensorBones {
		unsafe { &self.named }
	}

	#[inline(always)]
	fn raw_bones(&self) -> &[Vector4; BONE_COUNT] {
		unsafe { &self.raw }
	}
}
impl std::fmt::Debug for WinSdkKinectV1Skeleton {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("WinSdkKinectV1Skeleton").field("bones", &self.named_bones()).finish()
	}
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct NamedSensorBones {
	hip_center: Vector4,
	spine: Vector4,
	shoulder_center: Vector4,
	head: Vector4,
	shoulder_left: Vector4,
	elbow_left: Vector4,
	wrist_left: Vector4,
	hand_left: Vector4,
	shoulder_right: Vector4,
	elbow_right: Vector4,
	wrist_right: Vector4,
	hand_right: Vector4,
	hip_left: Vector4,
	knee_left: Vector4,
	ankle_left: Vector4,
	foot_left: Vector4,
	hip_right: Vector4,
	knee_right: Vector4,
	ankle_right: Vector4,
	foot_right: Vector4,
}

struct WinSdkKinectV1<U> {
	ptr: *mut c_void,
	thread: ManuallyDrop<std::thread::JoinHandle<()>>,
	_userdata: PhantomData<U>,
}
impl<U> WinSdkKinectV1<U> {
	#[inline]
	fn new(callback: WinSdkKinectV1Callback<U>, userdata: U) -> Result<Self, std::io::Error> {
		Self::new_(
			unsafe { core::mem::transmute::<_, CWinSdkKinectV1Callback>(callback) },
			Box::into_raw(Box::new(userdata)) as *mut c_void,
		)
	}

	fn new_(callback: CWinSdkKinectV1Callback, userdata: *mut c_void) -> Result<Self, std::io::Error> {
		let mut res = HRESULT(0);
		let ptr = unsafe { WinSdkKinectV1_Create(callback, userdata, &mut res) };
		if !ptr.is_null() && res.is_ok() {
			Ok(Self {
				ptr,

				thread: ManuallyDrop::new({
					let ptr = SendPtr(ptr);
					let userdata = SendPtr(userdata);
					std::thread::Builder::new()
						.name("rekinect_winsdk_v1".to_string())
						.spawn(move || unsafe {
							let ptr = { ptr };
							let ptr = ptr.0;
							if let Err(err) = WinSdkKinectV1_Run(ptr).ok() {
								log::error!("WinSdkKinectV1_Run() failed ({:?})", err);
							}
							WinSdkKinectV1_Destroy(ptr);

							let userdata = { userdata };
							let userdata = userdata.0;
							drop(Box::from_raw(userdata as *mut U));

							log::info!("WinSdkKinectV1 thread exited");
						})
						.unwrap()
				}),

				_userdata: PhantomData,
			})
		} else {
			Err(std::io::Error::new(
				std::io::ErrorKind::Other,
				format!("WinSdkKinectV1_Create() failed ({res:?})"),
			))
		}
	}
}
impl<U> Drop for WinSdkKinectV1<U> {
	fn drop(&mut self) {
		let thread = unsafe { ManuallyDrop::take(&mut self.thread) };
		unsafe {
			PostThreadMessageW(GetThreadId(HANDLE(thread.as_raw_handle() as isize)), WM_QUIT, WPARAM(0), LPARAM(0)).ok();
		}
		thread.join().ok();
	}
}

#[no_mangle]
pub extern "Rust" fn gmcl_rekinect_init(logger: &'static dyn log::Log) -> Result<Box<dyn KinectBackend>, std::io::Error> {
	log::set_logger(logger).ok();
	log::set_max_level(log::LevelFilter::Info);

	extern "C" fn callback(event: WinSdkKinectV1SkeletonUpdate, tx: &mut std::sync::mpsc::SyncSender<WinSdkKinectV1SkeletonUpdate>) {
		tx.send(event).ok();
	}

	let (tx, rx) = std::sync::mpsc::sync_channel(1);
	let kinect = WinSdkKinectV1::new(callback, tx)?;

	struct WinSdkKinectBackend {
		rx: std::sync::mpsc::Receiver<WinSdkKinectV1SkeletonUpdate>,
		skeleton: Option<usize>,
		inner: WinSdkKinectV1<std::sync::mpsc::SyncSender<WinSdkKinectV1SkeletonUpdate>>,
	}
	impl KinectBackend for WinSdkKinectBackend {
		fn poll(&mut self) -> Option<KinectSkeleton> {
			let event = self.rx.try_recv().ok()?;
			if self.skeleton.is_none() || self.skeleton == Some(event.skeleton_index) {
				if let Some(skeleton) = event.skeleton() {
					self.skeleton = Some(event.skeleton_index);

					let mut raw_bones = KinectSkeletonRawBones::default();

					skeleton.raw_bones().iter().zip(raw_bones.iter_mut()).for_each(|(src, dst)| {
						*dst = convert_kinect_coordinate_space_to_gmod(src);
					});

					return Some(KinectSkeleton::Tracked(KinectTrackedSkeleton::from_raw_bones(raw_bones)));
				} else if self.skeleton.is_some() {
					self.skeleton = None;
					return Some(KinectSkeleton::Untracked);
				}
			}
			None
		}

		#[inline]
		fn available(&self) -> bool {
			unsafe { WinSdkKinectV1_Available(self.inner.ptr) }
		}
	}

	Ok(Box::new(WinSdkKinectBackend {
		rx,
		inner: kinect,
		skeleton: None,
	}))
}
