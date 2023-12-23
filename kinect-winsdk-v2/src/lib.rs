#![cfg(windows)]

use kinect::{KinectBackend, KinectExtendedSkeletonBones, KinectSkeleton, KinectSkeletonBones, KinectTrackedExtendedSkeleton, KinectTrackedSkeleton};
use std::{
	ffi::c_void,
	marker::PhantomData,
	mem::ManuallyDrop,
	ops::{Add, Div},
	os::windows::io::AsRawHandle,
};
use windows::{
	core::HRESULT,
	Win32::{
		Foundation::{HANDLE, LPARAM, WPARAM},
		System::Threading::GetThreadId,
		UI::WindowsAndMessaging::{PostThreadMessageW, WM_QUIT},
	},
};

const BONE_COUNT: usize = 25;

#[inline]
fn convert_kinect_coordinate_space_to_gmod(vector: Vector3) -> [f32; 3] {
	[-vector.x, vector.z, vector.y]
}

#[link(name = "kinect_winsdk_v2_cpp", kind = "static")]
extern "C" {
	fn WinSdkKinectV2_Create(callback: CWinSdkKinectV2Callback, userdata: *mut c_void, res: &mut HRESULT) -> *mut c_void;
	fn WinSdkKinectV2_Destroy(ptr: *mut c_void);
	fn WinSdkKinectV2_Run(ptr: *mut c_void);
}

type CWinSdkKinectV2Callback = extern "C" fn(WinSdkKinectV2SkeletonUpdate, *mut c_void);
type WinSdkKinectV2Callback<U> = extern "C" fn(WinSdkKinectV2SkeletonUpdate, &mut U);

struct SendPtr<T>(*mut T);
unsafe impl<T> Send for SendPtr<T> {}
unsafe impl<T> Sync for SendPtr<T> {}

#[repr(C)]
struct WinSdkKinectV2SkeletonUpdate {
	skeleton_index: usize,
	skeleton: *const SensorBones,
}
impl WinSdkKinectV2SkeletonUpdate {
	#[inline]
	fn skeleton(&self) -> Option<&SensorBones> {
		if !self.skeleton.is_null() {
			Some(unsafe { &*self.skeleton })
		} else {
			None
		}
	}
}
impl std::fmt::Debug for WinSdkKinectV2SkeletonUpdate {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("WinSdkKinectV2SkeletonUpdate")
			.field("skeleton_index", &self.skeleton_index)
			.field("skeleton", &self.skeleton())
			.finish()
	}
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct Vector3 {
	x: f32,
	y: f32,
	z: f32,
}
impl Vector3 {
	#[inline]
	fn into_gmod(self) -> [f32; 3] {
		convert_kinect_coordinate_space_to_gmod(self)
	}
}
impl Add for Vector3 {
	type Output = Self;

	#[inline(always)]
	fn add(self, rhs: Self) -> Self::Output {
		Self {
			x: self.x + rhs.x,
			y: self.y + rhs.y,
			z: self.z + rhs.z,
		}
	}
}
impl Div<f32> for Vector3 {
	type Output = Self;

	#[inline(always)]
	fn div(self, rhs: f32) -> Self::Output {
		Self {
			x: self.x / rhs,
			y: self.y / rhs,
			z: self.z / rhs,
		}
	}
}

#[repr(C)]
union SensorBones {
	raw: [Vector3; BONE_COUNT],
	named: NamedSensorBones,
}
impl std::fmt::Debug for SensorBones {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		unsafe { self.named.fmt(f) }
	}
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct NamedSensorBones {
	spine_base: Vector3,
	spine_mid: Vector3,
	neck: Vector3,
	head: Vector3,
	shoulder_left: Vector3,
	elbow_left: Vector3,
	wrist_left: Vector3,
	hand_left: Vector3,
	shoulder_right: Vector3,
	elbow_right: Vector3,
	wrist_right: Vector3,
	hand_right: Vector3,
	hip_left: Vector3,
	knee_left: Vector3,
	ankle_left: Vector3,
	foot_left: Vector3,
	hip_right: Vector3,
	knee_right: Vector3,
	ankle_right: Vector3,
	foot_right: Vector3,
	spine_shoulder: Vector3,
	hand_tip_left: Vector3,
	thumb_left: Vector3,
	hand_tip_right: Vector3,
	thumb_right: Vector3,
}

struct WinSdkKinectV2<U> {
	thread: ManuallyDrop<std::thread::JoinHandle<()>>,
	_userdata: PhantomData<U>,
}
impl<U> WinSdkKinectV2<U> {
	#[inline]
	fn new(callback: WinSdkKinectV2Callback<U>, userdata: U) -> Result<Self, std::io::Error> {
		Self::new_(
			unsafe { core::mem::transmute::<_, CWinSdkKinectV2Callback>(callback) },
			Box::into_raw(Box::new(userdata)) as *mut c_void,
		)
	}

	fn new_(callback: CWinSdkKinectV2Callback, userdata: *mut c_void) -> Result<Self, std::io::Error> {
		let mut res = HRESULT(0);
		let ptr = unsafe { WinSdkKinectV2_Create(callback, userdata, &mut res) };
		if !ptr.is_null() && res.is_ok() {
			Ok(Self {
				thread: ManuallyDrop::new({
					let ptr = SendPtr(ptr);
					let userdata = SendPtr(userdata);
					std::thread::Builder::new()
						.name("rekinect_winsdk_v2".to_string())
						.spawn(move || unsafe {
							let ptr = { ptr };
							let ptr = ptr.0;
							WinSdkKinectV2_Run(ptr);
							WinSdkKinectV2_Destroy(ptr);

							let userdata = { userdata };
							let userdata = userdata.0;
							drop(Box::from_raw(userdata as *mut U));
						})
						.unwrap()
				}),

				_userdata: PhantomData,
			})
		} else {
			Err(std::io::Error::new(
				std::io::ErrorKind::Other,
				format!("WinSdkKinectV2_Create() failed ({res:?})"),
			))
		}
	}
}
impl<U> Drop for WinSdkKinectV2<U> {
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

	extern "C" fn callback(event: WinSdkKinectV2SkeletonUpdate, tx: &mut std::sync::mpsc::SyncSender<WinSdkKinectV2SkeletonUpdate>) {
		tx.send(event).ok();
	}

	let (tx, rx) = std::sync::mpsc::sync_channel(1);
	let kinect = WinSdkKinectV2::new(callback, tx)?;

	struct WinSdkKinectBackend {
		rx: std::sync::mpsc::Receiver<WinSdkKinectV2SkeletonUpdate>,
		skeleton: Option<usize>,
		_inner: WinSdkKinectV2<std::sync::mpsc::SyncSender<WinSdkKinectV2SkeletonUpdate>>,
	}
	impl KinectBackend for WinSdkKinectBackend {
		fn poll(&mut self) -> Option<KinectSkeleton> {
			let event = self.rx.try_recv().ok()?;
			if self.skeleton.is_none() || self.skeleton == Some(event.skeleton_index) {
				if let Some(skeleton) = event.skeleton() {
					self.skeleton = Some(event.skeleton_index);

					let bones = unsafe { &skeleton.named };

					return Some(KinectSkeleton::TrackedExtended(
						KinectTrackedSkeleton::from_named_bones(KinectSkeletonBones {
							spine: bones.spine_mid.into_gmod(),
							hip_center: ((bones.hip_left + bones.hip_right) / 2.0).into_gmod(),
							shoulder_center: ((bones.shoulder_left + bones.shoulder_right) / 2.0).into_gmod(),

							head: bones.head.into_gmod(),
							shoulder_left: bones.shoulder_left.into_gmod(),
							elbow_left: bones.elbow_left.into_gmod(),
							wrist_left: bones.wrist_left.into_gmod(),
							hand_left: bones.hand_left.into_gmod(),
							shoulder_right: bones.shoulder_right.into_gmod(),
							elbow_right: bones.elbow_right.into_gmod(),
							wrist_right: bones.wrist_right.into_gmod(),
							hand_right: bones.hand_right.into_gmod(),
							hip_left: bones.hip_left.into_gmod(),
							knee_left: bones.knee_left.into_gmod(),
							ankle_left: bones.ankle_left.into_gmod(),
							foot_left: bones.foot_left.into_gmod(),
							hip_right: bones.hip_right.into_gmod(),
							knee_right: bones.knee_right.into_gmod(),
							ankle_right: bones.ankle_right.into_gmod(),
							foot_right: bones.foot_right.into_gmod(),
						}),
						KinectTrackedExtendedSkeleton::from_named_bones(KinectExtendedSkeletonBones {
							hand_tip_left: bones.hand_tip_left.into_gmod(),
							thumb_left: bones.thumb_left.into_gmod(),
							hand_tip_right: bones.hand_tip_right.into_gmod(),
							thumb_right: bones.thumb_right.into_gmod(),
							neck: bones.neck.into_gmod(),
							spine_base: bones.spine_base.into_gmod(),
							spine_shoulder: bones.spine_shoulder.into_gmod(),
						}),
					));
				} else if self.skeleton.is_some() {
					self.skeleton = None;
					return Some(KinectSkeleton::Untracked);
				}
			}
			None
		}
	}

	Ok(Box::new(WinSdkKinectBackend {
		rx,
		_inner: kinect,
		skeleton: None,
	}))
}
