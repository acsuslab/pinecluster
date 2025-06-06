use std::fs::File;
use std::io::{BufRead, BufReader};
use std::process::Command;

fn main() {
    let file = File::open("nodelist").expect("Failed to open nodelist file");
    let reader = BufReader::new(file);

    for line in reader.lines() {
        if let Ok(raw_node) = line {
            let node_name = raw_node.trim();
            let node_addr = format!("node@{}.local", node_name);

            println!("=== Stopping event_beacon on {} ===", node_addr);

            let status = Command::new("ssh")
                .args([
                    "-i", "rpi.key",
                    "-o", "StrictHostKeyChecking=no",
                    &node_addr,
                    "pkill -9 -f event_beacon || true",
                ])
                .status();

            if let Err(e) = status {
                println!("Warning: could not stop event_beacon on {}: {}", node_addr, e);
            }
        }
    }

    println!("All nodes processed.");
}
