# Event Beacon

## About

This project implements a **storage‐free decentralized beacon network** that lets a private channel of clients execute smart contracts locally and only exchange lightweight provenance signatures (“**beacon stamps**”) instead of full state.

Each transaction request is broadcast to all nodes, which each verify the embedded client signature and only produce their beacon stamp once they’ve observed identical requests (same event hash) from every co-signing client, ensuring end-to-end trust.

By avoiding on-chain state storage and synchronizing only cryptographic proofs among consenting participants, it delivers high-throughput, scalable execution for permissioned smart-contract workflows.

---

## Table of Contents

- [Execution Steps](#execution-steps)
  - [Build (Release Mode)](#build-release-mode)
  - [Controller Mode (Remote Host)](#controller-mode-remote-host)
  - [Deploy to All Nodes](#deploy-to-all-nodes)
  - [Stop All Nodes](#stop-all-nodes)
  - [Transactions](#transactions)
- [File Summary](#file-summary)
- [General FAQs](#general-faqs)

---

## Execution Steps

### Build (Release Mode)

Compiles the project with maximum optimizations.

```bash
cargo build --release
```

### Controller Mode (Remote Host)

Deploy a host run with a client-request JSON and target node:

```bash
cargo run --bin event_beacon -- <request_from_client.json> <target_node_name>
```

Reads your signed client request, broadcasts it to all participants, gathers their BeaconStamps, assembles and signs the final receipt, logs it, writes it to `final_receipts/<event_hash>.json`, and records a Transaction.

### Deploy to All Nodes

Distribute the binary and start node servers:

```bash
cargo run --bin deploy_tool
```

### Stop All Nodes

SSH into each host (per `nodelist`) and kill any running beacon:

```bash
cargo run --bin stop_all_beacon
```

### Transactions

Each time a final receipt is generated in controller mode, a new Transaction is created:

| Field       | Description                                                                                          |
|-------------|------------------------------------------------------------------------------------------------------|
| **Payload** | The full signed receipt JSON                                                                         |
| **ID**      | SHA-256 over (payload + counter + prev_tx_hash)                                                      |
| **Stored**  | In an in-memory ledger and also written under `$HOME/Ebeacon/transactions/<tx_id>.json` on every node |

To inspect, simply run `ls $HOME/Ebeacon/transactions` or open files.

---

## File Summary

### Root directory (`Ebeacon/`)

| File | Description |
|------|-------------|
| `Cargo.toml` / `Cargo.lock` | Project dependencies and version lock file. |
| `deploy_tool.rs` | Stops any running beacon, copies binary & nodelist to each host, sets permissions, and starts node mode via SSH. |
| `stop_all_beacon.rs` | SSH’s into each host (per nodelist) and kills `event_beacon`. |
| `generate_keys.sh`, `generate_public_key.sh` | Helpers for RSA keypair generation. |
| `nodelist` | One node hostname per line—used by both deploy & stop tools. |
| `request_from_client.json` | Sample signed ClientRequest. |
| `rpi.key` / `rpi.pub` | SSH keys for deploying to Raspberry Pi nodes. |

### `src/`

| File | Description |
|------|-------------|
| `src/main.rs` | The CLI entry point: parses arguments, chooses “node” vs. “host” vs. “controller” mode, and hands off to `service.rs`. |
| `src/handler.rs` | Implements the HTTP API (using Warp) for `/clientrequest` and `/forward` endpoints: validates signatures, aggregates co-signers with a timeout, signs, logs, and responds with BeaconStamps. |
| `src/application.rs` | Coordinates “host” mode: reads a client request file, validates it, builds & signs a ForwardMessage, broadcasts it, collects all BeaconStamp replies, assembles & signs the final Receipt, logs it, and records a Transaction. |
| `src/service.rs` | Drives overall execution based on command-line mode flags—launches the Warp server for node mode or kicks off host/controller flows. |
| `src/action.rs` | Provides cryptographic utilities: nonce generation, signing raw JSON messages (ClientRequest, ForwardMessage, ReceiptPayload), and verifying responses. |
| `src/record.rs` | Persists any signed message (BeaconStamp or final Receipt) as a JSON file under `$HOME/Ebeacon/transactions`, using the transaction hash as filename. |
| `src/transaction.rs` | Defines the Transaction struct (ID, counter/timestamp, previous hash) and maintains an in-memory ledger of all Transactions recorded by the controller. |
| `src/network.rs` | Broadcasts the signed ForwardMessage via HTTP POST to each peer node (excluding self), applies an HTTP timeout, and gathers successful BeaconStamp responses. |
| `src/node.rs` | Implements all on-chain signature validations: checks ClientRequest signatures, ForwardMessage signatures, and incoming BeaconStamp payload integrity. |
| `src/host.rs` | Wraps up retrieval of the local host’s identity (e.g. system hostname) and exposes it to service/application layers. |
| `src/message.rs` | Houses the Rust structs and (de)serialization code for every JSON message type: ClientRequest, ForwardMessage, BeaconStamp, Receipt, etc. |
| `src/user.rs` & `src/role.rs` | Currently stubs for client-validation helpers and role definitions (Coordinator vs. Participant); placeholders for future access-control logic. |

---

## General FAQs

### Can I shorten or lengthen the HTTP timeout in `network.rs`?

Yes. That `timeout(Duration::from_secs(70), …)` controls how long the controller waits for each node’s `/forward` response. If you make it shorter than your nodes’ `AGG_WINDOW`, the controller may give up before they finish aggregating; if you make it much longer, the host will wait even after nodes have timed out or signed.

### Can I adjust the aggregation window in `handler.rs`?

Absolutely. The `AGG_WINDOW` in `/forward` sets how long each node waits for all co-signers before signing or erroring. A shorter window risks nodes timing out before slower peers reply; a longer window means the controller’s HTTP timeout must also be bumped up, or you’ll hit the HTTP cutoff first.

> **Tip:** Always keep your HTTP timeout ≥ your aggregation window, so the controller waits long enough for nodes to finish.

### How do I change the HTTP port used by nodes?

In `src/resource.rs`, the Warp server is started with a hard-coded port (e.g. `warp::serve(...).run(([0,0,0,0], 8080))`). Edit the second element of that tuple to your desired port, then recompile.

### Where do debug log lines (tracing) go and how can I suppress them?

Tracing calls (`tracing::debug!`) appear on stdout by default. You can either:

- Configure the tracing subscriber in `src/main.rs` or `src/service.rs` to filter out `DEBUG` level.
- Remove or comment out individual `debug!` calls in `handler.rs` and `application.rs`.

### I need to add a new CLI flag (e.g. `--verbose`). Where should I modify?

Edit `src/main.rs` (where argument parsing via `clap` or `std::env::args` happens). Add your flag there and pass its state into `service.rs` to influence logging or behavior.

### How do I change the behavior that decides between “node” and “controller” mode?

In `src/service.rs`, adjust the command-line argument matching logic (the `match args[..]` block) to recognize your new subcommands or flags, then call the appropriate `application::start_*` function.

### Where can I customize the directory used for saving “final_receipts”?

In `src/application.rs`, after logging the final receipt JSON, insert code such as:

```rust
fs::create_dir_all("final_receipts")?;
fs::write(
    format!("final_receipts/{}.json", hash),
    &final_receipt_json
)?;
```

Adjust the path string as needed before recompiling.

### How do I change the list of nodes I broadcast to?

In `start_host` within `src/application.rs`, the code reads `/home/node/Ebeacon/nodelist`. Change that hard-coded path to point at your desired list file.

### I want each node to log receipts but not return them to clients. Where should I change?

In `src/handler.rs`’s `handle_forward_action`, after signing the BeaconStamp, you currently `reply::json(&bs)`. To stop returning the JSON, change the handler to return only a status code (e.g. `StatusCode::OK`) or an empty body.

### How can I reset pending state upon a new request timestamp?

You’ve already added `batch_req_ts` & `completed_at_ts` fields in `PendingRequest`—tweak the comparison in step 4 of `handle_forward_action` so that if `req_ts > entry.batch_req_ts.unwrap_or(0)`, you clear pending, reset timestamps, and start a fresh batch.

### I need to swap from RSA to ECDSA signing. What changes?

In `src/action.rs`, replace RSA-based signing (`sign_data`, `sign_client_request_message`) with your ECDSA equivalents. In `src/utils.rs`, update key-loading to read EC keys instead of RSA PEMs.

### Where and how are transactions persisted?

`record::log_transaction(json: &str)` in `src/record.rs` writes each JSON blob to `$HOME/Ebeacon/transactions/<hash>.json`. To change the location, update the base path in that function (find the code using `dirs::home_dir()` and append your desired folder).

### How can I switch from file storage to a database?

Rewrite `log_transaction` in `src/record.rs` to send the JSON to your database client instead of writing to disk. All callers (handler and application) remain unaffected.

### I want to introduce per-node timeouts on HTTP requests.

In `src/network.rs`’s `broadcast_forward_request`, wrap each `client.post(...).send().await` in `tokio::time::timeout(your_duration, ...)` to enforce a per-node timeout.

### How can I add more stringent signature checks (e.g., key revocation)?

Modify `node_validate_client_request` and `node_validate_forward_message` in `src/node.rs` to consult a revocation list before returning `Ok(())`.

### I want to persist transactions beyond the in-memory ledger.

In `src/transaction.rs`’s `add_transaction`, after updating the `TRANSACTIONS` mutex, serialize and write the Transaction to disk or database.

### How do I add new fields to the JSON messages?

Update the appropriate struct in `src/message.rs` (e.g., `ForwardPayload`, `ReceiptPayload`) with your new `pub` field, derive `Serialize`/`Deserialize`, and bump any `"type"` strings if you version your protocol.

### I need a millisecond-precision timestamp instead of seconds. Where to edit?

Change `current_timestamp()` in `src/utils.rs` to use `SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis()` and adjust its return type to `u128` or `u64`.

### How can I change SSH credentials or user for deployment tools?

In `deploy_tool.rs` and `stop_all_beacon.rs`, locate the SSH command strings (e.g., `ssh -i key user@host`) and modify the `user@` portion or key path accordingly.

### Why do nodes sometimes wait the full aggregation timeout instead of returning receipts immediately?

Each node’s `/forward` handler blocks until either all expected co-signers have sent their ForwardMessage (and the batch is marked complete) or the `AGG_WINDOW` elapses. If every co-signer arrives early, the handler breaks out of the wait loop immediately and signs. Otherwise it waits up to the aggregation timeout before erroring with “cosigner timeout.” This behavior ensures no premature receipts are issued.

