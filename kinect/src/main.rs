use std::sync::atomic::AtomicBool;

static SHUTDOWN: AtomicBool = AtomicBool::new(false);

fn main() -> Result<(), std::io::Error> {
    ctrlc::set_handler(move || {
        if SHUTDOWN.swap(true, std::sync::atomic::Ordering::AcqRel) {
            println!("Aborting process");
            std::process::exit(1);
        }
    })
    .ok();

    let mut kinect = kinect::Kinect::new().unwrap();
    while !SHUTDOWN.load(std::sync::atomic::Ordering::Acquire) {
        if let Some(update) = kinect.poll() {
            println!("{:#?}", update);
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }

    println!("Shutdown gracefully");

    Ok(())
}
