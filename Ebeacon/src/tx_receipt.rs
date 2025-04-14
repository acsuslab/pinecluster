use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TxReceipt {
    pub tx_id: String,           // Transaction identifier (should match TxMessage's tx_id)
    pub ack: bool,               // True if the transaction is acknowledged (ACK); false for NACK
    pub receipt_info: String,    // Additional information (e.g., "Transaction successful", "Error: ...")
    pub timestamp: u64,          // Timestamp when the receipt was generated
}
