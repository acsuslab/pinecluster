1. EXECUTION STEPS
------------------

Build (Release Mode)  
  • `cargo build --release`  
    Compiles the project with maximum optimizations.

Controller Mode (Remote Host)  
  • Deploy a host run with a client-request JSON and target node:  
    ```  
    cargo run --bin event_beacon -- <request_from_client.json> <target_node_name>  
    ```  
  • Reads your signed client request, broadcasts it to all participants, gathers their BeaconStamps, assembles and signs the final receipt, logs it, writes it to `final_receipts/<event_hash>.json`, and records a Transaction.

Deploy to All Nodes  
  • Distribute the binary and start node servers:  
    ```  
    cargo run --bin deploy_tool  
    ```

Stop All Nodes  
  • SSH into each host (per `nodelist`) and kill any running beacon:  
    ```  
    cargo run --bin stop_all_beacon  
    ```

Transactions  
  • Each time a final receipt is generated in controller mode, a new Transaction is created:  
    – Payload: the full signed receipt JSON  
    – ID: SHA-256 over (payload + counter + prev_tx_hash)  
    – Stored in an in-memory ledger and also written under  
      `$HOME/Ebeacon/transactions/<tx_id>.json` on every node.  
  • To inspect, simply `ls $HOME/Ebeacon/transactions` or open files.

2. FILE SUMMARY
---------------

Root directory (Ebeacon/)  
  • Cargo.toml / Cargo.lock  
    Project dependencies and version lock file.

  • deploy_tool.rs  
    Stops any running beacon, copies binary & nodelist to each host, sets permissions, and starts node mode via SSH.

  • stop_all_beacon.rs  
    SSH’s into each host (per nodelist) and kills event_beacon.

  • generate_keys.sh, generate_public_key.sh  
    Helpers for RSA keypair generation.

  • nodelist  
    One node hostname per line—used by both deploy & stop tools.

  • request_from_client.json  
    Sample signed ClientRequest.

  • rpi.key / rpi.pub  
    SSH keys for deploying to Raspberry Pi nodes.

src/  
  • main.rs  
    CLI entry point dispatching to controller or node mode.

  • action.rs  
    Nonce generation and low-level sign/verify helpers produce BeaconStamp messages.

  • application.rs  
    “Host” logic: reads a client request, builds & signs a ForwardMessage, broadcasts it, collects BeaconStamps, builds & signs the final Receipt, logs it, writes it to final_receipts/, and records a Transaction.

  • handler.rs  
    Warp HTTP handlers for /clientrequest and /forward: validating signatures, aggregating co-signers, stamping & responding.

  • host.rs  
    Wraps getting this machine’s identity (hostname → public key path).

  • lib.rs  
    Re-exports modules for use as a library.

  • message.rs  
    All JSON-serializable structs: ClientRequest, ForwardMessage, BeaconStamp, Receipt, etc.

  • network.rs  
    Async broadcast & collect functions for forwarding and stamping.

  • node.rs  
    Signature validation logic for inbound ClientRequest and ForwardMessage.

  • record.rs  
    log_transaction(json: &str) writes the complete JSON of each BeaconStamp or Receipt to $HOME/Ebeacon/transactions/.

  • resource.rs  
    Sets up the Warp server routes for client requests and forwarding.

  • role.rs  
    Stub for future “Coordinator” vs “Participant” roles.

  • service.rs  
    Parses CLI args to start in node or controller mode, wires up the Host.

  • transaction.rs  
    Defines the Transaction struct (ID, counter, prev_hash) and an in-memory ledger.

  • tx_message.rs (optional)  
    Raw TxMessage definitions if you need generic deserialization.

  • tx_receipt.rs (optional)  
    JSON structs for acknowledgements (ACK/NACK) at the transaction layer.

  • user.rs  
    (stub) additional client key-validation helpers.

  • utils.rs  
    Misc helpers: current_timestamp(), key loading, path helpers, etc.


General FAQs
Q: Can I shorten or lengthen the HTTP timeout in network.rs?
A: Yes. That timeout(Duration::from_secs(70), …) controls how long the controller waits for each node’s /forward response. If you make it shorter than your nodes’ AGG_WINDOW, the controller may give up before they finish aggregating; if you make it much longer, the host will wait even after nodes have timed out or signed.

Q: Can I adjust the aggregation window in handler.rs?
A: Absolutely. The AGG_WINDOW in /forward sets how long each node waits for all co-signers before signing or erroring. A shorter window risks nodes timing out before slower peers reply; a longer window means the controller’s HTTP timeout must also be bumped up, or you’ll hit the HTTP cutoff first.
Tip: Always keep your HTTP timeout ≥ your aggregation window, so the controller waits long enough for nodes to finish.

Q: How do I change the HTTP port used by nodes?
A: In src/resource.rs, the Warp server is started with a hard-coded port (e.g. warp::serve(...).run(([0,0,0,0], 8080))). Edit the second element of that tuple to your desired port, then recompile.

Q: Where do debug log lines (tracing) go and how can I suppress them?
A: Tracing calls (tracing::debug!) appear on stdout by default. You can either:
Configure the tracing subscriber in src/main.rs or src/service.rs to filter out DEBUG level.
Remove or comment out individual debug! calls in handler.rs and application.rs.

Q: I need to add a new CLI flag (e.g. --verbose). Where should I modify?
A: Edit src/main.rs (where argument parsing via clap or std::env::args happens). Add your flag there and pass its state into service.rs to influence logging or behavior.

Q: How do I change the behavior that decides between “node” and “controller” mode?
A: In src/service.rs, adjust the command-line argument matching logic (the match args[..] block) to recognize your new subcommands or flags, then call the appropriate application::start_* function.

Q: Where can I customize the directory used for saving “final_receipts”?
A: In src/application.rs, after logging the final receipt JSON, insert code such as:
fs::create_dir_all("final_receipts")?;
fs::write(
    format!("final_receipts/{}.json", hash),
    &final_receipt_json
)?;
Adjust the path string as needed before recompiling.

Q: How do I change the list of nodes I broadcast to?
A: In start_host within src/application.rs, the code reads /home/node/Ebeacon/nodelist. Change that hard-coded path to point at your desired list file.

Q: I want each node to log receipts but not return them to clients. Where should I change?
A: In src/handler.rs’s handle_forward_action, after signing the BeaconStamp, you currently reply::json(&bs). To stop returning the JSON, change the handler to return only a status code (e.g. StatusCode::OK) or an empty body.

Q: How can I reset pending state upon a new request timestamp?
A: You’ve already added batch_req_ts & completed_at_ts fields in PendingRequest—tweak the comparison in step 4 of handle_forward_action so that if req_ts > entry.batch_req_ts.unwrap_or(0), you clear pending, reset timestamps, and start a fresh batch.

Q: I need to swap from RSA to ECDSA signing. What changes?
A: In src/action.rs, replace RSA-based signing (sign_data, sign_client_request_message) with your ECDSA equivalents. In src/utils.rs, update key-loading to read EC keys instead of RSA PEMs.

Q: Where and how are transactions persisted?
A: record::log_transaction(json: &str) in src/record.rs writes each JSON blob to $HOME/Ebeacon/transactions/<hash>.json. To change the location, update the base path in that function (find the code using dirs::home_dir() and append your desired folder).

Q: How can I switch from file storage to a database?
A: Rewrite log_transaction in src/record.rs to send the JSON to your database client instead of writing to disk. All callers (handler and application) remain unaffected.

Q: I want to introduce per-node timeouts on HTTP requests.
A: In src/network.rs’s broadcast_forward_request, wrap each client.post(...).send().await in tokio::time::timeout(your_duration, ...) to enforce a per-node timeout.

Q: How can I add more stringent signature checks (e.g., key revocation)?
A: Modify node_validate_client_request and node_validate_forward_message in src/node.rs to consult a revocation list before returning Ok(()).

Q: I want to persist transactions beyond the in-memory ledger.
A: In src/transaction.rs’s add_transaction, after updating the TRANSACTIONS mutex, serialize and write the Transaction to disk or database.

Q: How do I add new fields to the JSON messages?
A: Update the appropriate struct in src/message.rs (e.g., ForwardPayload, ReceiptPayload) with your new pub field, derive Serialize/Deserialize, and bump any "type" strings if you version your protocol.

Q: I need a millisecond-precision timestamp instead of seconds. Where to edit?
A: Change current_timestamp() in src/utils.rs to use SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis() and adjust its return type to u128 or u64.

Q: How can I change SSH credentials or user for deployment tools?
A: In deploy_tool.rs and stop_all_beacon.rs, locate the SSH command strings (e.g., ssh -i key user@host) and modify the user@ portion or key path accordingly.

Q: Why do nodes sometimes wait the full aggregation timeout instead of returning receipts immediately?
A: Each node’s /forward handler blocks until either all expected co-signers have sent their ForwardMessage (and the batch is marked complete) or the AGG_WINDOW elapses. If every co-signer arrives early, the handler breaks out of the wait loop immediately and signs. Otherwise it waits up to the aggregation timeout before erroring with “cosigner timeout.” This behavior ensures no premature receipts are issued.
