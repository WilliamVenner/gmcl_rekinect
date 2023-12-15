use std::{
    ffi::c_void,
    marker::PhantomData,
    mem::{ManuallyDrop, MaybeUninit},
    os::windows::io::AsRawHandle,
};
use windows::{
    core::HRESULT,
    Win32::{
        Foundation::{LPARAM, WPARAM},
        UI::WindowsAndMessaging::{PostThreadMessageW, WM_QUIT},
    },
};

#[link(name = "gm_kinect_v1", kind = "static")]
extern "C" {
    fn KinectV1_Create(callback: CKinectV1Callback, userdata: *mut c_void) -> *mut c_void;
    fn KinectV1_Destroy(ptr: *mut c_void);
    fn KinectV1_Run(ptr: *mut c_void) -> HRESULT;
}

type CKinectV1Callback = extern "C" fn(KinectV1SkeletonUpdate, *mut c_void);
pub type KinectV1Callback<U> = extern "C" fn(KinectV1SkeletonUpdate, &mut U);

struct SendPtr<T>(*mut T);
unsafe impl<T> Send for SendPtr<T> {}
unsafe impl<T> Sync for SendPtr<T> {}

#[repr(C)]
pub struct KinectV1SkeletonUpdate {
    pub skeleton_index: usize,
    pub state: SkeletonTrackingState,
    pos: MaybeUninit<SkeletonPos>,
}
impl KinectV1SkeletonUpdate {
    #[inline]
    pub fn pos(&self) -> Option<KinectV1Skeleton> {
        match self.state {
            SkeletonTrackingState::NotTracked => None,
            SkeletonTrackingState::PositionOnly => Some(KinectV1Skeleton::PositionOnly(unsafe {
                self.pos.assume_init_ref().pos_only
            })),
            SkeletonTrackingState::Tracked => Some(KinectV1Skeleton::Tracked(unsafe {
                self.pos.assume_init_ref().tracked
            })),
        }
    }
}
impl std::fmt::Debug for KinectV1SkeletonUpdate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KinectV1SkeletonUpdate")
            .field("skeleton_index", &self.skeleton_index)
            .field("state", &self.state)
            .field("pos", &self.pos())
            .finish()
    }
}

#[repr(C)]
union SkeletonPos {
    pos_only: SkeletonPositionOnly,
    tracked: SkeletonTracked,
}

#[derive(Clone, Copy)]
#[repr(C)]
pub struct SkeletonPositionOnly {
    pos: *const Vector4,
}
impl SkeletonPositionOnly {
    #[inline(always)]
    pub fn pos(&self) -> &Vector4 {
        unsafe { &*self.pos }
    }
}
impl std::fmt::Debug for SkeletonPositionOnly {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SkeletonPositionOnly")
            .field("pos", &self.pos())
            .finish()
    }
}

#[derive(Clone, Copy)]
#[repr(C)]
pub struct SkeletonTracked {
    pos: *const Vector4,
    bones: *const SensorBones,
}
impl SkeletonTracked {
    #[inline(always)]
    pub fn pos(&self) -> &Vector4 {
        unsafe { &*self.pos }
    }

    #[inline(always)]
    pub fn bones(&self) -> &NamedSensorBones {
        unsafe { &(*self.bones).named }
    }

    #[inline(always)]
    pub fn raw_bones(&self) -> &[Vector4; 20] {
        unsafe { &(*self.bones).raw }
    }
}
impl std::fmt::Debug for SkeletonTracked {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SkeletonTracked")
            .field("pos", &self.pos())
            .field("bones", &self.bones())
            .finish()
    }
}

#[derive(Debug, Clone, Copy)]
pub enum KinectV1Skeleton {
    PositionOnly(SkeletonPositionOnly),
    Tracked(SkeletonTracked),
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub enum SkeletonTrackingState {
    NotTracked = 0,
    PositionOnly = 1,
    Tracked = 2,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Vector4 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

#[repr(C)]
union SensorBones {
    raw: [Vector4; 20],
    named: NamedSensorBones,
}
impl std::fmt::Debug for SensorBones {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        unsafe { self.named.fmt(f) }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct NamedSensorBones {
    pub hip_center: Vector4,
    pub spine: Vector4,
    pub shoulder_center: Vector4,
    pub head: Vector4,
    pub shoulder_left: Vector4,
    pub elbow_left: Vector4,
    pub wrist_left: Vector4,
    pub hand_left: Vector4,
    pub shoulder_right: Vector4,
    pub elbow_right: Vector4,
    pub wrist_right: Vector4,
    pub hand_right: Vector4,
    pub hip_left: Vector4,
    pub knee_left: Vector4,
    pub ankle_left: Vector4,
    pub foot_left: Vector4,
    pub hip_right: Vector4,
    pub knee_right: Vector4,
    pub ankle_right: Vector4,
    pub foot_right: Vector4,
}

pub struct KinectV1<U> {
    thread: ManuallyDrop<std::thread::JoinHandle<()>>,
    _userdata: PhantomData<U>,
}
impl<U> KinectV1<U> {
    #[inline]
    pub fn new(callback: KinectV1Callback<U>, userdata: U) -> Result<Self, std::io::Error> {
        Self::new_(
            unsafe { core::mem::transmute::<_, CKinectV1Callback>(callback) },
            Box::into_raw(Box::new(userdata)) as *mut c_void,
        )
    }

    fn new_(callback: CKinectV1Callback, userdata: *mut c_void) -> Result<Self, std::io::Error> {
        let ptr = unsafe { KinectV1_Create(callback, userdata) };
        if !ptr.is_null() {
            Ok(Self {
                thread: ManuallyDrop::new({
                    let ptr = SendPtr(ptr);
                    let userdata = SendPtr(userdata);
                    std::thread::Builder::new()
                        .name("gm_kinect_v1".to_string())
                        .spawn(move || unsafe {
                            let ptr = { ptr };
                            let ptr = ptr.0;
                            let _ = KinectV1_Run(ptr);
                            KinectV1_Destroy(ptr);

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
                "KinectV1_Create() failed",
            ))
        }
    }
}
impl<U> Drop for KinectV1<U> {
    fn drop(&mut self) {
        let thread = unsafe { ManuallyDrop::take(&mut self.thread) };
        unsafe {
            PostThreadMessageW(
                dbg!(thread.as_raw_handle() as usize as _),
                WM_QUIT,
                WPARAM(0),
                LPARAM(0),
            )
            .ok();
        }
    }
}
