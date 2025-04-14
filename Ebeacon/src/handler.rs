/*
 * src/handler.rs
 * --------------------------------------------------------------------------
 * Provides HTTP endpoints using Warp.
 * - "/clientrequest": Validates a complete ClientRequest and logs it.
 * - "/forward": Validates the forward message?s signature; then, validates
 *   and signs its embedded ClientRequest to produce and log a BeaconStamp.
 * --------------------------------------------------------------------------
 */

use crate::action;
use crate::record;
use crate::message::{ForwardMessage, ClientRequest};
use crate::utils::{load_private_key, load_public_key};
use warp::http::StatusCode;
use warp::reply;
use warp::Reply;
use warp::Rejection;
use serde_json;

// Import the node-level validation functions.
use crate::node::{node_validate_client_request, node_validate_forward_message};
use std::fmt;

// Custom error type for rejecting requests.
pub struct MyReject(String);
impl warp::reject::Reject for MyReject {}

impl fmt::Debug for MyReject {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "MyReject: {}", self.0)
    }
}

impl fmt::Display for MyReject {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Handles incoming client requests by validating the signature of the full ClientRequest.
/// On success, logs and returns the ClientRequest as a JSON response.
pub async fn handle_client_request(req: ClientRequest) -> Result<impl Reply, Rejection> {
    match node_validate_client_request(&req) {
         Ok(()) => {
              // Log the validated request.
              let client_req_json = serde_json::to_string_pretty(&req)
                   .unwrap_or_default();
              record::log_transaction(&client_req_json);
              Ok(warp::reply::with_status(reply::json(&req), StatusCode::OK))
         },
         Err(err) => {
              eprintln!("Client request signature validation failed: {}", err);
              Err(warp::reject::custom(MyReject(err)))
         }
    }
}

/// Processes a forward message by first validating its signature.
/// Then extracts the embedded ClientRequest JSON, converts it both to a 
/// ClientRequest (for validation) and a serde_json::Value (for signing),
/// validates the embedded ClientRequest, signs it to create a BeaconStamp,
/// logs the BeaconStamp, and returns it as a JSON response.
pub async fn handle_forward_action(forward: ForwardMessage) -> Result<impl Reply, Rejection> {
    // Validate the forward message's signature.
    match node_validate_forward_message(&forward) {
         Ok(()) => {
              // Extract the embedded client request JSON.
              let client_request_json = forward.message.payload.client_request.clone();
    
              // Parse into a ClientRequest for validation.
              let client_req: ClientRequest = serde_json::from_str(&client_request_json)
                   .map_err(|e| {
                       eprintln!("Error parsing client request JSON: {}", e);
                       warp::reject::custom(MyReject("Parsing error".to_string()))
                   })?;
    
              // Validate the embedded ClientRequest's signature.
              match node_validate_client_request(&client_req) {
                   Ok(()) => {
                        // Parse into a serde_json::Value for signing.
                        let client_request_value: serde_json::Value = serde_json::from_str(&client_request_json)
                            .map_err(|e| {
                                eprintln!("Error converting client request JSON to Value: {}", e);
                                warp::reject::custom(MyReject("Parsing error".to_string()))
                            })?;
                        
                        let public_key = load_public_key();
                        let private_key = load_private_key();
                        let timestamp = forward.message.payload.timestamp;
                        
                        // Sign the embedded client request.
                        let beaconstamp = action::sign_client_request_message(
                            &client_request_value,
                            timestamp,
                            &private_key,
                            &public_key,
                        )
                        .map_err(|e| {
                             eprintln!("Error signing client request: {}", e);
                             warp::reject::custom(MyReject("Signing error".to_string()))
                        })?;
                        
                        // Log the resulting BeaconStamp.
                        let beaconstamp_json = serde_json::to_string_pretty(&beaconstamp)
                             .unwrap_or_default();
                        record::log_transaction(&beaconstamp_json);
                        
                        Ok(warp::reply::with_status(reply::json(&beaconstamp), StatusCode::OK))
                   },
                   Err(err) => {
                        eprintln!("Validation error in embedded client request: {}", err);
                        Err(warp::reject::custom(MyReject(err)))
                   }
              }
         },
         Err(err) => {
              eprintln!("Forward message signature validation failed: {}", err);
              Err(warp::reject::custom(MyReject(err)))
         }
    }
}
