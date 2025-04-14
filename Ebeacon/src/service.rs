/*
 * src/service.rs
 * -------------------------------------------------------------
 * Manages runtime mode based on CLI arguments and deploys the
 * event_beacon service accordingly. It handles node mode (server)
 * and host mode (processing client requests locally or remotely).
 * -------------------------------------------------------------
 */

use std::env;
use crate::application;
use std::process::Command;
use crate::host::Host;

pub async fn run_service() {
    let args: Vec<String> = env::args().collect();

    // Case 1: Node mode (run server)
    if args.len() == 2 && args[1] == "node" {
        println!("Starting node service...");
        application::start_node().await;
    }
    // Case 2: Controller mode: JSON file and target node provided.
    else if args.len() == 3 && args[1].ends_with(".json") {
        let file_name = args[1].clone();
        let target_node = args[2].clone();
        println!("Starting host service using file '{}' targeting node '{}'", file_name, target_node);
        remote_execute(file_name, target_node);
    }
    // Case 3: Local host mode: only a JSON file provided.
    else if args.len() == 2 && args[1].ends_with(".json") {
        let file_name = args[1].clone();
        let host = Host::new();
        let self_name = host.id().to_string();
        println!("Starting host service using file '{}' on node '{}'", file_name, self_name);
        application::start_host_from_file(file_name, self_name).await;
    } else {
        // Print usage instructions.
        println!("Usage:");
        println!("  event_beacon node");
        println!("    - Runs a node server at port 8080");
        println!("  event_beacon <request_from_client.json> <target_node_name>");
        println!("    - (Controller Mode) Deploys host mode remotely and returns the final receipt.");
        println!("  event_beacon <request_from_client.json>");
        println!("    - (Local Host Mode) Runs host mode on the current node using the provided JSON file.");
    }
}

/// Executes commands on a remote node via SSH (for deployment purposes).
fn remote_execute(file_name: String, target_node: String) {
    // Define the remote path to the Ebeacon folder.
    let remote_directory = format!("node@{}.local:~/Ebeacon/", target_node);

    // Copy the JSON file (client request) to the remote node.
    let scp_status = Command::new("scp")
        .arg("-i")
        .arg("rpi.key")
        .arg(&file_name)
        .arg(&remote_directory)
        .status()
        .expect("failed to execute scp");
    if !scp_status.success() {
        eprintln!("SCP failed for node {}", target_node);
        return;
    }

    // Execute the host-mode command on the remote node.
    let ssh_command = format!("cd ~/Ebeacon && ./event_beacon \"{}\"", file_name);
    let ssh_output = Command::new("ssh")
        .arg("-i")
        .arg("rpi.key")
        .arg(format!("node@{}.local", target_node))
        .arg(ssh_command)
        .output()
        .expect("failed to execute ssh");

    if !ssh_output.status.success() {
        eprintln!(
            "Remote execution failed on node {}: {}",
            target_node,
            String::from_utf8_lossy(&ssh_output.stderr)
        );
    } else {
        println!(
            "Remote output from {}:\n{}",
            target_node,
            String::from_utf8_lossy(&ssh_output.stdout)
        );
    }
}
