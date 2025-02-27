use std::{
    fs::File,
    io::{self, BufRead},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

fn main() -> io::Result<()> {
    let nodes = read_nodes("nodelist")?;
    
    for node in nodes {
        println!("🔄 Processing node: {}", node);
        match update_node(&node) {
            Ok(_) => println!("✅ Successfully updated {}", node),
            Err(e) => eprintln!("❌ Failed to update {}: {}", node, e),
        }
    }
    Ok(())
}

fn update_node(node: &str) -> io::Result<()> {
    // 1. Stop existing service
    ssh_execute(node, "pkill -f start_service || true")?;

    // 2. Archive previous version
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    
    ssh_execute(node, &format!(
        "mkdir -p ~/cluster/versions && \
         mv ~/cluster/start_service ~/cluster/versions/start_service_{} 2>/dev/null || true", 
        timestamp
    ))?;

    // 3. Upload new binary
    Command::new("scp")
        .args([
            "-i", "rpi.key",
            "-o", "StrictHostKeyChecking=no",
            "target/release/start_service",
            &format!("{}:~/cluster/", node)
        ])
        .status()?;

    // 4. Set permissions and start new service
    ssh_execute(node, "chmod +x ~/cluster/start_service")?;
    ssh_execute(node, "nohup ~/cluster/start_service > ~/cluster/service.log 2>&1 &")?;

    // 5. Verify deployment
    if check_service_running(node) {
        println!("🟢 Service running on {}", node);
    } else {
        eprintln!("🔴 Service NOT running on {}", node);
    }

    Ok(())
}

// Helper functions
fn read_nodes(path: &str) -> io::Result<Vec<String>> {
    let file = File::open(path)?;
    io::BufReader::new(file)
        .lines()
        .map(|l| l.map(|s| s.trim().to_string()))
        .collect()
}

fn ssh_execute(node: &str, cmd: &str) -> io::Result<()> {
    Command::new("ssh")
        .args([
            "-i", "rpi.key",
            "-o", "StrictHostKeyChecking=no",
            node,
            cmd
        ])
        .status()?;
    Ok(())
}

fn check_service_running(node: &str) -> bool {
    let output = Command::new("ssh")
        .args([
            "-i", "rpi.key",
            "-o", "StrictHostKeyChecking=no",
            node,
            "pgrep -f start_service"
        ])
        .output();

    match output {
        Ok(o) => !o.stdout.is_empty(),
        Err(_) => false,
    }
}


