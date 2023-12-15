pub use kinect_v1::KinectV1Skeleton as Skeleton;

pub const SKELETON_COUNT: usize = 6;
pub const SKELETON_BONE_COUNT: usize = 20;

extern "C" fn kinect_callback(
    update: kinect_v1::KinectV1SkeletonUpdate,
    userdata: &mut std::sync::mpsc::SyncSender<kinect_v1::KinectV1SkeletonUpdate>,
) {
    userdata.send(update).ok();
}

pub struct Kinect {
    rx: std::sync::mpsc::Receiver<kinect_v1::KinectV1SkeletonUpdate>,
    _kinect: kinect_v1::KinectV1<std::sync::mpsc::SyncSender<kinect_v1::KinectV1SkeletonUpdate>>,
}
impl Kinect {
    pub fn new() -> Result<Self, std::io::Error> {
        let (tx, rx) = std::sync::mpsc::sync_channel(1);
        let kinect = kinect_v1::KinectV1::new(kinect_callback, tx)?;
        Ok(Self {
            _kinect: kinect,
            rx,
        })
    }

    pub fn poll(&mut self) -> Option<kinect_v1::KinectV1SkeletonUpdate> {
        self.rx.try_recv().ok()
    }
}
