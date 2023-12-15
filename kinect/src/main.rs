fn main() -> Result<(), std::io::Error> {
    #[cfg(windows)]
    {
        let mut kinect = kinect::Kinect::new().unwrap();
        loop {
            if let Some(update) = kinect.poll() {
                println!("{:#?}", update);
            }
        }
    }
}
