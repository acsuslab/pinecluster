use serde::{Serialize, Deserialize};
use serde_json::Value;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TxMessage {
    pub tx_id: String,           // Unique transaction identifier (e.g., a hash)
    pub from: String,            // Sender's public key
    #[serde(rename = "type")]
    pub message_type: String,    // Type of message: "clientrequest", "forward", or "beaconstamp"
    pub content: Value,          // Message payload in a flexible JSON format
    pub signature: String,       // Cryptographic signature of the message
}
