/*
 * src/network.rs
 * -------------------------------------------------------------
 * Broadcasts forward messages to remote nodes asynchronously using HTTP.
 * Collects BeaconStamp responses from nodes and returns them as a vector.
 * -------------------------------------------------------------
 */

use crate::message::BeaconStamp;
use crate::message::ForwardMessage;
use reqwest::Client;
use tokio::time::{timeout, Duration};
use futures::future::join_all;

/// Broadcasts a forward message to all nodes (except the sender) and collects BeaconStamp responses.
///
/// # Arguments
/// - `forward`: The forward message to broadcast.
/// - `self_name`: The identifier of the sending node.
/// - `nodes`: A list of node names in the network.
///
/// # Returns
/// A vector of BeaconStamp responses received from nodes.
pub async fn broadcast_forward_request(
    forward: ForwardMessage,
    self_name: String,
    nodes: Vec<String>,
) -> Vec<BeaconStamp> {
    let client = Client::new();

    let futures = nodes
        .into_iter()
        .filter(|node| node != &self_name)
        .map(|node| {
            let url = format!("http://{}.local:8080/forward", node);
            let forward_clone = forward.clone();
            let client = client.clone();
            async move {
                // Debug: show where we're sending and what payload
                eprintln!(
                    "[DEBUG] Broadcasting forward to {} with payload:\n{}",
                    url,
                    serde_json::to_string_pretty(&forward_clone)
                        .unwrap_or_else(|_| "<invalid JSON>".into())
                );

                // Send the request with a timeout
                match timeout(Duration::from_secs(70), client.post(&url).json(&forward_clone).send()).await {
                    // Request completed within timeout
                    Ok(send_result) => match send_result {
                        // HTTP request succeeded
                        Ok(resp) => {
                            eprintln!("[DEBUG] Received {} from {}", resp.status(), url);
                            // Try to parse the JSON body
                            match resp.json::<BeaconStamp>().await {
                                Ok(bs) => {
                                    eprintln!("[DEBUG] Parsed BeaconStamp from {}", node);
                                    Some(bs)
                                }
                                Err(e) => {
                                    eprintln!("[ERROR] Failed to parse BeaconStamp from {}: {}", node, e);
                                    None
                                }
                            }
                        }
                        // HTTP request failed
                        Err(e) => {
                            eprintln!("[ERROR] HTTP error sending to {}: {}", url, e);
                            None
                        }
                    },
                    // Timeout occurred
                    Err(_) => {
                        eprintln!("[ERROR] Timeout sending to {}", url);
                        None
                    }
                }
            }
        });

    // Await all sends concurrently and collect successful responses
    let results = join_all(futures).await;
    results.into_iter().flatten().collect()
}
