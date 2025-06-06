// src/application.rs
// -------------------------------------------------------------
// Coordinates the overall transaction process.
// In host mode, it reads a client request file, validates its signature,
// builds a fully?signed forward message, broadcasts it to nodes, collects
// BeaconStamp responses, builds and signs the final receipt, logs it, and
// records an official Transaction entry.
// -------------------------------------------------------------

use std::fs;
use crate::message::{
    ClientRequest,
    ForwardMessage,
    ForwardPayload,
    ForwardMessageBody,
    Receipt,
    ReceiptMessage,
    ReceiptPayload,
};
use crate::utils::{current_timestamp, load_public_key, load_private_key};
use crate::network::broadcast_forward_request;
use crate::action::{generate_nonce, sign_data};
use crate::record;
use crate::transaction::{Transaction, add_transaction, TRANSACTIONS};
use crate::node::node_validate_client_request;
use crate::resource;

/// Starts the node HTTP server on port 8080.
pub async fn start_node() {
    println!("Node mode: starting server...");
    resource::start_server().await;
}

/// Reads, validates, and parses a client request JSON file, then kicks off host mode.
pub async fn start_host_from_file(file_name: String, self_name: String) {
    let client_request_json = fs::read_to_string(&file_name)
        .unwrap_or_else(|e| panic!("Failed to read {}: {}", file_name, e));
    let client_req: ClientRequest = serde_json::from_str(&client_request_json)
        .unwrap_or_else(|e| panic!("Failed to parse JSON from {}: {}", file_name, e));

    // Validate the client's signature on their request.
    match node_validate_client_request(&client_req) {
        Ok(()) => {
            let event_hash = client_req.message.payload.event_hash.clone();
            println!(
                "Extracted event hash '{}' from file '{}'",
                event_hash, file_name
            );
            start_host(client_request_json, event_hash, self_name).await;
fs::remove_file(&file_name).unwrap_or_else(|e| eprintln!("Warning: failed to delete {}: {}", file_name, e));

        }
        Err(err) => {
            eprintln!("Invalid client request signature: {}", err);
            panic!("Aborting host mode.");
        }
    }
}

/// Builds, signs, and broadcasts a forward message, gathers BeaconStamps,
/// then builds, signs, logs, and records the final receipt.
pub async fn start_host(client_request_json: String, hash: String, self_name: String) {
    println!(
        "Host mode: coordinating hash '{}' on node '{}'",
        hash, self_name
    );

    // 1) Construct the forward message body.
    let forward_body = ForwardMessageBody {
        r#type: "forward".to_string(),
        from: load_public_key(),
        nonce: generate_nonce(),
        payload: ForwardPayload {
            event_hash: hash.clone(),
            timestamp: current_timestamp(),
            client_request: client_request_json.clone(),
        },
    };

    // 2) Canonical JSON and sign it.
    let forward_body_json = serde_json::to_string(&forward_body)
        .unwrap_or_else(|_| "".to_string());
    let private_key = load_private_key();
    let forward_signature = sign_data(&forward_body_json, &private_key)
        .unwrap_or_else(|_| "ERROR".to_string());

    let forward = ForwardMessage {
        message: forward_body,
        signature: forward_signature,
    };

    // 3) Broadcast to all other nodes.
    let nodes: Vec<String> = match fs::read_to_string("/home/node/Ebeacon/nodelist") {
        Ok(content) => content.lines().map(String::from).collect(),
        Err(e) => {
            eprintln!("Error reading nodelist: {}", e);
            vec![]
        }
    };
    let responses = broadcast_forward_request(forward, self_name.clone(), nodes).await;

    // 4) Assemble the receipt.
    let receipt_payload = ReceiptPayload {
        timestamp: current_timestamp(),
        beaconstamps: responses,
    };
    let receipt_message = ReceiptMessage {
        r#type: "receipt".to_string(),
        from: load_public_key(),
        nonce: generate_nonce(),
        payload: receipt_payload,
    };

    // 5) Sign the receipt JSON.
    let receipt_message_json = serde_json::to_string_pretty(&receipt_message)
        .unwrap_or_else(|_| "".to_string());
    let receipt_signature = sign_data(&receipt_message_json, &private_key)
        .unwrap_or_else(|_| "ERROR".to_string());

    let receipt = Receipt {
        message: receipt_message,
        signature: receipt_signature,
    };

    // 6) Log and display the final receipt.
    let final_receipt_json = serde_json::to_string_pretty(&receipt)
        .unwrap_or_else(|_| "".to_string());
    record::log_transaction(&final_receipt_json);
    println!("Final Receipt:\n{}", final_receipt_json);

    // 7) Create and record the official Transaction.
    let counter = current_timestamp();
    let previous_hash = TRANSACTIONS
        .lock()
        .unwrap()
        .values()
        .last()
        .map(|tx| tx.id.clone());
    let tx = Transaction::new(&final_receipt_json, counter, previous_hash.as_deref());
    add_transaction(&tx);
    println!("Transaction recorded with ID: {}", tx.id);


}
