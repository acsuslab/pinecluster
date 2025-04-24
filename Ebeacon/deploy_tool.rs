/*
 * Deploys the event_beacon binary to all nodes listed in the "nodelist" file.
 * Stops any existing instance, copies the binary and nodelist, sets permissions,
 * and starts the event_beacon in node mode remotely via SSH.
 */

use std::{
    fs::File,
    io::{self, BufRead, BufReader},
    process::Command,
    thread,
    time::Duration,
};

fn main() -> io::Result<()> {
    // Open the nodelist file and load node names.
    let file = File::open("nodelist")?;
    let nodes = BufReader::new(file)
        .lines()
        .collect::<Result<Vec<_>, _>>()?;

    for node in nodes {
        let node = node.trim();
        if node.is_empty() {
            continue;
        }
        let ssh_addr = format!("node@{}.local", node);
        println!("=== Deploying & Starting on {} ===", ssh_addr);

        // Step 1: Stop any existing event_beacon process.
        if let Err(e) = stop_old_process(&ssh_addr) {
            eprintln!("Failed to stop process on {}: {}", ssh_addr, e);
        }

        // Step 2: Ensure the remote Ebeacon folder exists.
        run_ssh(&ssh_addr, &["mkdir -p ~/Ebeacon"])?;

        // Step 3: Copy the event_beacon binary to the remote node.
        run_scp(&ssh_addr, "target/release/event_beacon", "~/Ebeacon/")?;

        // Step 4: Copy the nodelist file to the remote node.
        run_scp(&ssh_addr, "nodelist", "~/Ebeacon/")?;

        // Step 5: Set execution permissions on the binary.
        run_ssh(&ssh_addr, &["chmod +x ~/Ebeacon/event_beacon"])?;

        // Step 6: Start the event_beacon in node mode using nohup.
        run_ssh(
            &ssh_addr,
            &["bash -c \"nohup ~/Ebeacon/event_beacon node > ~/Ebeacon/node.log 2>&1 &\""],
        )?;

        println!("=== Done with {} ===\n", ssh_addr);
        thread::sleep(Duration::from_secs(1));
    }
    println!("All nodes processed.");
    Ok(())
}

fn stop_old_process(ssh_addr: &str) -> io::Result<()> {
    // Use "|| true" so that the command returns 0 even if pkill finds no match.
    let stop_cmd = "pkill -f event_beacon || true";
    if let Err(e) = run_ssh(ssh_addr, &[&format!("bash -c '{}'", stop_cmd)]) {
        eprintln!("Warning: {} -> Failed to stop old process: {}", ssh_addr, e);
        println!("Retrying using manual process check...");
        if let Err(e) = force_kill_process(ssh_addr) {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                format!("Failed to kill process: {}", e),
            ));
        }
    }
    Ok(())
}

fn force_kill_process(ssh_addr: &str) -> io::Result<()> {
    // Find the PID of the running event_beacon process.
    let check_process_cmd = "ps aux | grep event_beacon | grep -v grep | awk '{print $2}'";
    let full_check_cmd = format!("bash -c '{}'", check_process_cmd);
    let output = Command::new("ssh")
        .args(&["-i", "rpi.key", "-o", "StrictHostKeyChecking=no", ssh_addr, &full_check_cmd])
        .output()?;
    let pid = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if !pid.is_empty() {
        println!("Found event_beacon process with PID: {}", pid);
        let kill_cmd = format!("kill -9 {}", pid);
        let full_kill_cmd = format!("bash -c '{}'", kill_cmd);
        run_ssh(ssh_addr, &[&full_kill_cmd])?;
        println!("Successfully killed process with PID: {}", pid);
    } else {
        println!("No event_beacon process found to kill.");
    }
    Ok(())
}

fn run_ssh(ssh_addr: &str, commands: &[&str]) -> io::Result<()> {
    // Join multiple commands with " && " to execute them sequentially.
    let remote_cmd = commands.join(" && ");
    let status = Command::new("ssh")
        .args(&[
            "-i", "rpi.key",
            "-o", "StrictHostKeyChecking=no",
            ssh_addr,
            remote_cmd.as_str(),
        ])
        .status()?;
    
    // For our stop command, ignore failure; for others, warn if nonzero.
    if remote_cmd.contains("pkill -f event_beacon") {
        // Do nothing.
    } else if !status.success() {
        eprintln!("Warning: command failed on {} => {}", ssh_addr, remote_cmd);
    }
    Ok(())
}

fn run_scp(ssh_addr: &str, local_file: &str, remote_dir: &str) -> io::Result<()> {
    let dest = format!("{}:{}", ssh_addr, remote_dir);
    let status = Command::new("scp")
        .args(&[
            "-i", "rpi.key",
            "-o", "StrictHostKeyChecking=no",
            local_file,
            &dest,
        ])
        .status()?;
    if !status.success() {
        eprintln!("Warning: scp failed for {} -> {}", local_file, dest);
    }
    Ok(())
}
