// src/transaction.rs
// -------------------------------------------------------------
// Official representation of a state transition of a resource.
// Each Transaction has a unique identifier (transaction hash computed from
// its payload, counter, and previous transaction's hash), a counter (for ordering),
// and an optional previous_hash for linking transactions.
// A global ledger of transactions is maintained.
// -------------------------------------------------------------
#![allow(dead_code)]

use std::collections::HashMap;
use std::sync::Mutex;
use sha2::{Sha256, Digest};
use lazy_static::lazy_static;

#[derive(Debug, Clone)]
pub struct Transaction {
    /// Unique transaction identifier (hash)
    pub id: String,
    /// Counter to help order transactions (or sequence number)
    pub counter: u64,
    /// Optional hash of the previous transaction (None if this is the first)
    pub previous_hash: Option<String>,
    /// The transaction payload as a JSON string (represents the state transition)
    pub payload: String,
}

impl Transaction {
    /// Creates a new Transaction.
    /// Computes the transaction hash as the SHA?256 of "payload:counter:previous_hash".
    pub fn new(payload: &str, counter: u64, previous_hash: Option<&str>) -> Self {
        let prev = previous_hash.unwrap_or("");
        let input = format!("{}:{}:{}", payload, counter, prev);
        let mut hasher = Sha256::new();
        hasher.update(input.as_bytes());
        let result = hasher.finalize();
        let id = format!("{:x}", result);
        Transaction {
            id,
            counter,
            previous_hash: previous_hash.map(|s| s.to_string()),
            payload: payload.to_string(),
        }
    }
}

lazy_static! {
    /// Global ledger mapping transaction IDs to Transactions.
    pub static ref TRANSACTIONS: Mutex<HashMap<String, Transaction>> = Mutex::new(HashMap::new());
}

/// Adds a transaction to the global ledger.
pub fn add_transaction(tx: &Transaction) {
    let mut ledger = TRANSACTIONS.lock().unwrap();
    ledger.insert(tx.id.clone(), tx.clone());
}

/// Updates a transaction in the ledger (stub for future expansion).
pub fn update_transaction(id: &str, new_payload: &str) {
    let mut ledger = TRANSACTIONS.lock().unwrap();
    if let Some(tx) = ledger.get_mut(id) {
        tx.payload = new_payload.to_string();
    }
}

/// Retrieves a transaction from the ledger by id.
pub fn get_transaction(id: &str) -> Option<Transaction> {
    let ledger = TRANSACTIONS.lock().unwrap();
    ledger.get(id).cloned()
}
