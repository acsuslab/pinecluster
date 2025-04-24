// src/node.rs
// -------------------------------------------------------------
// Provides generic signature validation for any serializable message,
// along with convenience functions to validate ClientRequest, BeaconStamp, and ForwardMessage.
// -------------------------------------------------------------
use serde::Serialize;
use sha2::{Sha256, Digest};
use rsa::{
    RsaPublicKey,
    pkcs8::DecodePublicKey,
    pkcs1v15::{VerifyingKey, Signature},
    signature::Verifier,
};
use base64::{engine::general_purpose, Engine as _};
use serde_json;
use crate::message::{ClientRequest, ForwardMessage};

pub fn validate_signature<T: Serialize>(
    message: &T,
    signature_b64: &str,
    public_key_pem: &str,
) -> Result<(), String> {
    let message_json = serde_json::to_string(message)
        .map_err(|e| format!("Serialization error: {}", e))?;
    
    let mut hasher = Sha256::new();
    hasher.update(message_json.as_bytes());
    let hash_result = hasher.finalize();
    
    let public_key = RsaPublicKey::from_public_key_pem(public_key_pem)
        .map_err(|e| format!("Public key parse error: {}", e))?;
    let verifying_key = VerifyingKey::<Sha256>::new(public_key);
    
    let sig_bytes = general_purpose::STANDARD
        .decode(signature_b64)
        .map_err(|e| format!("Base64 decode error: {}", e))?;
    
    let signature = Signature::try_from(sig_bytes.as_slice())
        .map_err(|_| "Signature conversion failed".to_string())?;
    
    verifying_key.verify(&hash_result, &signature)
        .map_err(|e| format!("Signature verification error: {}", e))
}

/// Validates a ClientRequest by verifying the signature on its "message" field
/// using the public key in that field.
pub fn node_validate_client_request(client_request: &ClientRequest) -> Result<(), String> {
    validate_signature(
        &client_request.message,
        &client_request.signature,
        &client_request.message.from,
    )
}



/// Validates a ForwardMessage by verifying the signature on its "message" field
/// using the public key found in that field.  
/// This ensures that the forward message itself is signed correctly.
pub fn node_validate_forward_message(forward: &ForwardMessage) -> Result<(), String> {
    validate_signature(
        &forward.message,
        &forward.signature,
        &forward.message.from,
    )
}
