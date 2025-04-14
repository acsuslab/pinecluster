// src/application.rs
// -------------------------------------------------------------
// Coordinates the overall transaction process.
// In host mode, it reads a client request file, validates its signature,
// builds a forward message (now fully signed) that includes the complete client request JSON,
// broadcasts it to nodes, collects BeaconStamp responses, and then builds
// a final receipt which is signed and logged.
// After producing the final receipt, an official Transaction is created
// from its JSON representation.
// -------------------------------------------------------------
use std::fs;
use crate::message::{ClientRequest, Receipt, ReceiptMessage, ReceiptPayload, ForwardMessage, ForwardPayload, ForwardMessageBody};
use crate::utils::{current_timestamp, load_public_key, load_private_key};
use crate::network::broadcast_forward_request;
use crate::action;
use crate::action::generate_nonce;
use crate::record;
use crate::transaction::{Transaction, add_transaction, TRANSACTIONS};
use crate::node::node_validate_client_request;

pub async fn start_node() {
    println!("Node mode: starting server...");
    crate::resource::start_server().await;
}

pub async fn start_host_from_file(file_name: String, self_name: String) {
    let client_request_json = fs::read_to_string(&file_name)
         .unwrap_or_else(|e| panic!("Failed to read {}: {}", file_name, e));
    let client_req: ClientRequest = serde_json::from_str(&client_request_json)
         .unwrap_or_else(|e| panic!("Failed to parse JSON from {}: {}", file_name, e));
    
    // Validate the client request signature.
    match node_validate_client_request(&client_req) {
         Ok(()) => {
             let event_hash = client_req.message.payload.event_hash;
             println!("Extracted event hash '{}' from file '{}'", event_hash, file_name);
             start_host(client_request_json, event_hash, self_name).await;
         },
         Err(err) => {
             eprintln!("Validation of client request failed: {}", err);
             panic!("Invalid client request signature. Aborting host mode.");
         }
    }
}

/// Builds a forward message that includes the complete client request JSON,
/// signs it, broadcasts it to nodes, collects BeaconStamp responses, and then
/// builds a final receipt which is signed and logged. It then creates an official
/// Transaction record from the final receipt JSON.
pub async fn start_host(client_request_json: String, hash: String, self_name: String) {
    println!("Host mode: starting coordination with hash '{}' on node '{}'", hash, self_name);
    
    // Build the forward message body.
    let forward_body = ForwardMessageBody {
         r#type: "forward".to_string(),
         from: load_public_key(),
         nonce: action::generate_nonce(),
         payload: ForwardPayload {
             event_hash: hash.clone(),
             timestamp: current_timestamp(),
             client_request: client_request_json.clone(),
         },
    };
    
    // Canonicalize the forward message body for signing.
    let forward_body_json = serde_json::to_string(&forward_body)
         .unwrap_or_else(|_| "".to_string());
    let private_key = load_private_key();
    let forward_signature = action::sign_data(&forward_body_json, &private_key)
         .unwrap_or_else(|_| "ERROR".to_string());
    
    // Build the full forward message with its signature.
    let forward = ForwardMessage {
         message: forward_body,
         signature: forward_signature,
    };

    let nodes: Vec<String> = match fs::read_to_string("/home/node/Ebeacon/nodelist") {
         Ok(content) => content.lines().map(String::from).collect(),
         Err(e) => {
              eprintln!("Error reading nodelist: {}", e);
              vec![]
         }
    };

    let responses = broadcast_forward_request(forward, self_name.clone(), nodes).await;

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

    let receipt_message_json = serde_json::to_string_pretty(&receipt_message)
         .unwrap_or_else(|_| "".to_string());
    let receipt_signature = action::sign_data(&receipt_message_json, &private_key)
         .unwrap_or_else(|_| "ERROR".to_string());

    let receipt = Receipt {
         message: receipt_message,
         signature: receipt_signature,
    };

    let final_receipt_json = serde_json::to_string_pretty(&receipt)
         .unwrap_or_else(|_| "".to_string());
    record::log_transaction(&final_receipt_json);

    println!("Final Receipt:\n{}", final_receipt_json);

    let payload = final_receipt_json.clone();
    let counter = current_timestamp();
    let previous_hash: Option<String> = {
        let ledger = TRANSACTIONS.lock().unwrap();
        ledger.values().last().map(|tx| tx.id.clone())
    };

    let tx = Transaction::new(&payload, counter, previous_hash.as_deref());
    add_transaction(&tx);

    println!("Transaction recorded with ID: {}", tx.id);
}
