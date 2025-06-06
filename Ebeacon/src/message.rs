// src/message.rs
// -------------------------------------------------------------
// Defines the JSON structures for the messages exchanged in the system.
// Every message has a "message" object containing:
//   - "type": e.g., "clientrequest", "beaconstamp", "forward", "receipt"
//   - "from": the sender's public key
//   - "nonce": a unique random string
//   - "payload": an object holding the actual data (its structure depends on the message type)
// 
// For a client request, the payload includes { event_hash, timestamp }.
// For a forward message, the payload includes { event_hash, timestamp, client_request }.
// For a BeaconStamp, the payload includes { timestamp, signedmessage }.
// For a receipt, the payload includes { timestamp, beaconstamps }.
// -------------------------------------------------------------
use serde::{Serialize, Deserialize};
use serde_json;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ClientRequestPayload {
    pub event_hash: String,
    pub timestamp: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ClientRequestMessage {
    #[serde(rename = "type")]
    pub r#type: String, // expected "clientrequest"
    pub from: String,
    pub cosigners: Vec<String>,
    pub nonce: String,
    pub payload: ClientRequestPayload,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ClientRequest {
    pub message: ClientRequestMessage,
    pub signature: String,
}

/// Forward message payload contains the full client request JSON.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ForwardPayload {
    pub event_hash: String,
    pub timestamp: u64,
    pub client_request: String, // complete JSON from client
}

/// The actual forward message body.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ForwardMessageBody {
    #[serde(rename = "type")]
    pub r#type: String, // e.g., "forward"
    pub from: String,
    pub nonce: String,
    pub payload: ForwardPayload,
}

/// A complete forward message, including its signature.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ForwardMessage {
    pub message: ForwardMessageBody,
    pub signature: String,
}

/// BeaconStamp payload stores the signed client request as a JSON value (for pretty printing).
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BeaconStampPayload {
    pub timestamp: u64,
    pub signedmessage: serde_json::Value,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BeaconStampMessage {
    #[serde(rename = "type")]
    pub r#type: String, // expected "beaconstamp"
    pub from: String,
    pub nonce: String,
    pub payload: BeaconStampPayload,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BeaconStamp {
    pub message: BeaconStampMessage,
    pub signature: String,
}

/// Receipt payload contains an array of BeaconStamp responses.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ReceiptPayload {
    pub timestamp: u64,
    pub beaconstamps: Vec<BeaconStamp>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ReceiptMessage {
    #[serde(rename = "type")]
    pub r#type: String, // expected "receipt"
    pub from: String,
    pub nonce: String,
    pub payload: ReceiptPayload,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Receipt {
    pub message: ReceiptMessage,
    pub signature: String,
}
