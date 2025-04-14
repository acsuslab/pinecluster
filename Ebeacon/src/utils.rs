use chrono::Utc;
use std::fs;

pub fn current_timestamp() -> u64 {
    Utc::now().timestamp() as u64
}

pub fn load_public_key() -> String {
    fs::read_to_string("/home/node/Ebeacon/public.pem")
        .unwrap_or_else(|e| {
            eprintln!("Error reading public key: {}", e);
            "unknown_public_key".to_string()
        })
}

pub fn load_private_key() -> String {
    fs::read_to_string("/home/node/Ebeacon/private.pem")
        .unwrap_or_else(|e| {
            eprintln!("Error reading private key: {}", e);
            "".to_string()
        })
}
