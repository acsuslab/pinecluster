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
pub async fn broadcast_forward_request(forward: ForwardMessage, self_name: String, nodes: Vec<String>) -> Vec<BeaconStamp> {
    let client = Client::new(); // Create an HTTP client.
    
    // Prepare futures for each node (excluding the sender).
    let futures = nodes.into_iter().filter(|node| node != &self_name)
         .map(|node| {
              // Build the destination URL.
              let url = format!("http://{}.local:8080/forward", node);
              let forward_clone = forward.clone();
              let client = client.clone();
              async move {
                  // Send the forward message with a 1-second timeout.
                  match timeout(Duration::from_secs(1), client.post(&url).json(&forward_clone).send()).await {
                      Ok(Ok(resp)) => {
                          // Attempt to parse the response as a BeaconStamp.
                          match resp.json::<BeaconStamp>().await {
                              Ok(bs) => Some(bs),
                              Err(e) => {
                                  eprintln!("Error parsing response from {}: {}", node, e);
                                  None
                              }
                          }
                      },
                      _ => {
                          eprintln!("No response from {}", node);
                          None
                      }
                  }
              }
         });
    // Await all futures concurrently and collect non-None results.
    let results = join_all(futures).await;
    results.into_iter().flatten().collect()
}
