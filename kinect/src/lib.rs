use std::{io::Write, mem::ManuallyDrop};

pub use kinect_v1::KinectV1Skeleton as Skeleton;

pub const SKELETON_COUNT: usize = 6;
pub const SKELETON_BONE_COUNT: usize = 20;

extern "C" fn kinect_callback(
    update: kinect_v1::KinectV1SkeletonUpdate,
    userdata: &mut std::sync::mpsc::SyncSender<kinect_v1::KinectV1SkeletonUpdate>,
) {
    std::fs::OpenOptions::new()
        .append(true)
        .write(true)
        .truncate(false)
        .create(true)
        .open("kinect.txt")
        .unwrap()
        .write_all(format!("{update:#?}").as_bytes())
        .unwrap();
    userdata.send(update).ok();
}

pub struct Kinect {
    thread: ManuallyDrop<std::thread::JoinHandle<()>>,
    rx: std::sync::mpsc::Receiver<kinect_v1::KinectV1SkeletonUpdate>,
}
impl Kinect {
    pub fn new() -> Result<Self, std::io::Error> {
        let (tx, rx) = std::sync::mpsc::sync_channel(1);

        let kinect = kinect_v1::KinectV1::new(kinect_callback, tx)?;
        let thread = std::thread::Builder::new()
            .name("gm_kinect".to_string())
            .spawn(move || {
                if let Err(err) = kinect.run() {
                    eprintln!("Kinect error: {err:?}");
                }
            })
            .unwrap();

        Ok(Self {
            thread: ManuallyDrop::new(thread),
            rx,
        })
    }

    pub fn poll(&mut self) -> Option<kinect_v1::KinectV1SkeletonUpdate> {
        self.rx.try_recv().ok()
    }
}
impl Drop for Kinect {
    fn drop(&mut self) {
        let _thread = unsafe { ManuallyDrop::take(&mut self.thread) };
        // TODO kill and join thread
    }
}
