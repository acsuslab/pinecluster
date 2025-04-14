// src/action.rs
// -------------------------------------------------------------
// Responsible for signing data. This version includes a function to
// sign the complete client request JSON (passed as a serde_json::Value)
// that is forwarded. The resulting BeaconStamp will have a payload containing
// the current timestamp and the field "signedmessage" set to the client request
// as a JSON value. A random nonce is generated for each message.
// -------------------------------------------------------------
extern crate rand;
use rand::Rng;
use rsa::{RsaPrivateKey, pkcs8::DecodePrivateKey};
use rsa::pkcs1v15::SigningKey;
use rsa::signature::Signer;
use sha2::{Sha256, Digest};
use base64::{engine::general_purpose, Engine as _};
use serde_json;
use crate::message::{BeaconStamp, BeaconStampMessage, BeaconStampPayload};

/// Generate a random nonce as a string.
pub fn generate_nonce() -> String {
    let nonce: u64 = rand::thread_rng().gen();
    nonce.to_string()
}


pub fn sign_client_request_message(
    client_request_value: &serde_json::Value,
    timestamp: u64,
    private_key_pem: &str,
    from: &str,
) -> Result<BeaconStamp, String> {
    let nonce = generate_nonce();
    
    // Create canonical JSON (compact format) from the parsed client request.
    let canonical_json = serde_json::to_string(client_request_value)
         .map_err(|e| format!("Error serializing client request in compact form: {}", e))?;
    
    // Compute the SHA?256 hash of the compact JSON.
    let mut hasher = Sha256::new();
    hasher.update(canonical_json.as_bytes());
    let hash_result = hasher.finalize();
    
    // Parse the private key.
    let private_key = RsaPrivateKey::from_pkcs8_pem(private_key_pem)
         .map_err(|e| format!("PEM parse error: {}", e))?;
    let signing_key = SigningKey::<Sha256>::new(private_key);
    let signature = signing_key.sign(&hash_result);
    let signature_b64 = general_purpose::STANDARD.encode(signature.as_ref());
    
    // Build the BeaconStamp message with the parsed JSON value stored in "signedmessage".
    let beaconstamp_msg = BeaconStampMessage {
        r#type: "beaconstamp".to_string(),
        from: from.to_string(),
        nonce,
        payload: BeaconStampPayload {
            timestamp,
            signedmessage: client_request_value.clone(),
        },
    };
    
    Ok(BeaconStamp {
        message: beaconstamp_msg,
        signature: signature_b64,
    })
}

/// Signs arbitrary data (helper, e.g., for receipt messages).
pub fn sign_data(data: &str, private_key_pem: &str) -> Result<String, String> {
    let private_key = RsaPrivateKey::from_pkcs8_pem(private_key_pem)
         .map_err(|e| format!("PEM parse error: {}", e))?;
    let hashed = Sha256::digest(data.as_bytes());
    let signing_key = SigningKey::<Sha256>::new(private_key);
    let signature = signing_key.sign(&hashed);
    Ok(general_purpose::STANDARD.encode(signature.as_ref()))
}
