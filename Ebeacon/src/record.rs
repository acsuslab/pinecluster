// src/record.rs
// -------------------------------------------------------------
// Persists a transaction?s complete JSON (e.g., a signed BeaconStamp or Receipt)
// as an individual file in "$HOME/Ebeacon/transactions". The filename is derived
// from the SHA-256 hash of the JSON content.
// -------------------------------------------------------------

use std::fs;
use std::path::PathBuf;
use sha2::{Sha256, Digest};
use home::home_dir;

/// Retrieves the base directory for Ebeacon ("$HOME/Ebeacon").
/// Falls back to "./Ebeacon" if the home directory cannot be determined.
fn get_ebeacon_dir() -> PathBuf {
    if let Some(home) = home_dir() {
        home.join("Ebeacon")
    } else {
        eprintln!("Unable to detect home directory. Defaulting to './Ebeacon'");
        PathBuf::from("Ebeacon")
    }
}

/// Logs the complete JSON data of a transaction by writing it to a file.
/// The file is stored in "$HOME/Ebeacon/transactions" and is named using the
/// SHA-256 hash (hexadecimal) of the data with a `.json` extension.
pub fn log_transaction(data: &str) {
    // Determine the base Ebeacon directory.
    let ebeacon_dir = get_ebeacon_dir();
    // Define the transactions directory.
    let transactions_dir = ebeacon_dir.join("transactions");

    println!("Attempting to create transactions directory at: {}", transactions_dir.display());

    // Create the directory (and parents) if it does not exist.
    if let Err(e) = fs::create_dir_all(&transactions_dir) {
        eprintln!("Failed to create transactions directory at {}: {}", transactions_dir.display(), e);
        return;
    } else {
        println!("Directory exists or was created: {}", transactions_dir.display());
    }

    // Compute the SHA-256 hash of the data.
    let mut hasher = Sha256::new();
    hasher.update(data.as_bytes());
    let hash_result = hasher.finalize();
    let filename = format!("{:x}.json", hash_result);
    let filepath = transactions_dir.join(&filename);

    println!("Writing transaction file to: {}", filepath.display());

    // Write the JSON data to the file.
    match fs::write(&filepath, data) {
        Ok(_) => println!("Transaction logged successfully to: {}", filepath.display()),
        Err(e) => eprintln!("Error writing transaction to {}: {}", filepath.display(), e),
    }
}
