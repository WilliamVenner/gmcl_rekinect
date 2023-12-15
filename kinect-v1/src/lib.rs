use std::{ffi::c_void, marker::PhantomData, mem::MaybeUninit};
use windows::core::HRESULT;

#[link(name = "gm_kinect_v1", kind = "static")]
extern "C" {
    fn KinectV1_Create(callback: CKinectV1Callback, userdata: *mut c_void) -> *mut c_void;
    fn KinectV1_Destroy(ptr: *mut c_void);
    fn KinectV1_Run(ptr: *mut c_void) -> HRESULT;
    fn KinectV1_UserData(ptr: *mut c_void) -> *mut c_void;
}

type CKinectV1Callback = extern "C" fn(KinectV1SkeletonUpdate, *mut c_void);
pub type KinectV1Callback<U> = extern "C" fn(KinectV1SkeletonUpdate, &mut U);

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

#[repr(transparent)]
pub struct KinectV1<U> {
    ptr: *mut c_void,
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
                ptr,
                _userdata: PhantomData,
            })
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "KinectV1_Create() failed",
            ))
        }
    }

    pub fn run(self) -> Result<(), std::io::Error> {
        unsafe { KinectV1_Run(self.ptr) }.ok()?;
        Ok(())
    }
}
impl<U> Drop for KinectV1<U> {
    fn drop(&mut self) {
        if !self.ptr.is_null() {
            unsafe {
                let userdata = KinectV1_UserData(self.ptr) as *mut U;
                if !userdata.is_null() {
                    drop(Box::from_raw(userdata));
                }

                KinectV1_Destroy(self.ptr);
            }
        }
    }
}
unsafe impl<U> Send for KinectV1<U> {}
