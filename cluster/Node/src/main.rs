use std::{thread, time::Duration};

fn main() {
    let version = env!("CARGO_PKG_VERSION");
    loop {
        println!("[v{}] Service running", version);
        thread::sleep(Duration::from_secs(60));
    }
}
