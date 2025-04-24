// src/handler.rs
// -------------------------------------------------------------
// - /clientrequest: Validate & log a signed ClientRequest.
// - /forward:     Validate the signed ForwardMessage, then wait
//                up to 60s for all co?signers, sign/log/respond.
// -------------------------------------------------------------

use crate::action;
use crate::message::{BeaconStamp, ClientRequest, ForwardMessage};
use crate::node::{node_validate_client_request, node_validate_forward_message};
use crate::record;
use crate::utils::{load_private_key, load_public_key};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::sync::Arc;
use tokio::sync::{Mutex, Notify};
use tokio::time::{sleep, Duration, Instant};
use warp::{http::StatusCode, reply, Rejection, Reply};
use once_cell::sync::Lazy;

/// Tracks expected vs. arrived co?signers, and remembers
/// per?batch timestamps so we can reset on new requests.
struct PendingRequest {
    expected:        HashSet<String>,
    pending:         HashSet<String>,
    batch_req_ts:    Option<u64>,
    completed_at_ts: Option<u64>,
    notify:          Arc<Notify>,
}

/// Global pending map: event_hash ? PendingRequest
static PENDING: Lazy<Mutex<HashMap<String, PendingRequest>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

/// 60?second aggregation window
const AGG_WINDOW: Duration = Duration::from_secs(60);

/// Custom rejection type
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

/// /clientrequest handler: validate & log
pub async fn handle_client_request(req: ClientRequest) -> Result<impl Reply, Rejection> {
    node_validate_client_request(&req)
        .map_err(|e| warp::reject::custom(MyReject(e)))?;
    record::log_transaction(&serde_json::to_string_pretty(&req).unwrap_or_default());
    Ok(warp::reply::with_status(reply::json(&req), StatusCode::OK))
}

/// /forward handler: validate, aggregate co?signers, sign & respond
pub async fn handle_forward_action(forward: ForwardMessage) -> Result<impl Reply, Rejection> {
    let event_hash = forward.message.payload.event_hash.clone();
    let req_ts = forward.message.payload.timestamp;

    // 1) Validate forward signature
    node_validate_forward_message(&forward)
        .map_err(|e| warp::reject::custom(MyReject(e)))?;
    tracing::debug!("[{}] forward sig OK", &event_hash);

    // 2) Parse & validate embedded client request
    let cr_json = &forward.message.payload.client_request;
    let client_req: ClientRequest = serde_json::from_str(cr_json)
        .map_err(|e| warp::reject::custom(MyReject(format!("parse clientreq: {}", e))))?;
    node_validate_client_request(&client_req)
        .map_err(|e| warp::reject::custom(MyReject(e)))?;
    tracing::debug!("[{}] embedded clientreq OK", &event_hash);

    // 3) Build expected set = { original sender } ? cosigners
    let mut expected: HashSet<String> =
        client_req.message.cosigners.iter().cloned().collect();
    expected.insert(client_req.message.from.clone());

    // 4) Insert or reset entry if this is a fresh batch
    {
        let mut map = PENDING.lock().await;
        let entry = map.entry(event_hash.clone())
            .or_insert_with(|| PendingRequest {
                expected:        expected.clone(),
                pending:         HashSet::new(),
                batch_req_ts:    None,
                completed_at_ts: None,
                notify:          Arc::new(Notify::new()),
            });

        // If we've already completed a batch at ts < this req_ts, start a new batch
        if let Some(batch_ts) = entry.batch_req_ts {
            if req_ts > batch_ts {
                tracing::debug!("[{}] detected new batch ({} > {}), resetting",
                    &event_hash, req_ts, batch_ts);
                entry.pending.clear();
                entry.batch_req_ts = None;
                entry.completed_at_ts = None;
            }
        }

        // Update expected & record our own arrival
        entry.expected = expected.clone();
        entry.pending.insert(client_req.message.from.clone());
        // wake up any waiter
        entry.notify.notify_waiters();
        tracing::debug!(
            "[{}] pending {}/{}: {:?}",
            &event_hash,
            entry.pending.len(),
            entry.expected.len(),
            entry.pending
        );
    }

    // 5) Wait up to AGG_WINDOW for either:
    //    - this batch to be marked complete (completed_at_ts == req_ts), or
    //    - us to see that pending == expected and become the marker.
    let notify = {
        let map = PENDING.lock().await;
        map.get(&event_hash).unwrap().notify.clone()
    };
    let deadline = Instant::now() + AGG_WINDOW;

    loop {
        {
            let map = PENDING.lock().await;
            if let Some(entry) = map.get(&event_hash) {
                // Unblock if this batch is already marked complete
                if entry.completed_at_ts == Some(req_ts) {
                    tracing::debug!("[{}] batch {} already completed, unblocking", &event_hash, req_ts);
                    break;
                }
                // If we're the first to see everyone, mark the batch complete
                if entry.pending == entry.expected {
                    drop(map); // release read lock
                    let mut map = PENDING.lock().await;
                    if let Some(entry) = map.get_mut(&event_hash) {
                        entry.batch_req_ts = Some(req_ts);
                        entry.completed_at_ts = Some(req_ts);
                        tracing::debug!(
                            "[{}] all co?signers in for batch {}, marking complete",
                            &event_hash,
                            req_ts
                        );
                        entry.notify.notify_waiters();
                    }
                    break;
                }
            }
        }

        // Timeout?
        let now = Instant::now();
        if now >= deadline {
            let mut map = PENDING.lock().await;
            map.remove(&event_hash);
            tracing::error!("[{}] timeout waiting for co?signers", &event_hash);
            return Err(warp::reject::custom(MyReject("cosigner timeout".into())));
        }

        // wait for either a new arrival or the remaining timeout
        let to_wait = deadline - now;
        tokio::select! {
            _ = notify.notified() => {},
            _ = sleep(to_wait) => {},
        }
    }

    // 6) Now: everyone signs *their own* receipt for this batch
    let bs: BeaconStamp = {
        let value: Value = serde_json::from_str(cr_json)
            .map_err(|e| warp::reject::custom(MyReject(format!("json?Value: {}", e))))?;
        let pk = load_public_key();
        let sk = load_private_key();
        action::sign_client_request_message(&value, req_ts, &sk, &pk)
            .map_err(|e| warp::reject::custom(MyReject(e)))?
    };

    // 7) Log our stamp and return it
    record::log_transaction(&serde_json::to_string_pretty(&bs).unwrap_or_default());
    Ok(warp::reply::with_status(reply::json(&bs), StatusCode::OK))
}
