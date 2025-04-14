Execution Steps:

Build (Release Mode)
cargo build --release
Compiles the project with optimizations.

Running the Application
Node Mode (Server):
Start the node server (listens on port 8080) with:
cargo run --bin event_beacon node

Controller Mode (Remote Host):
Deploy host mode remotely using a client request JSON file and target node name:
cargo run --bin event_beacon -- <request_from_client.json> <target_node_name>
This copies the JSON file to the specified node, starts host mode remotely, and returns a final receipt.

Deployment & Stopping
Deploy to Nodes:
Use the deploy tool to distribute the binary and start the service on all nodes:
cargo run --bin deploy_tool
Stop All Nodes:
Use the stop tool to terminate any running event_beacon processes on all nodes:
cargo run --bin stop_all_beacon
Transactions
When a final receipt is generated (in controller mode), a Transaction record is created by:

Using the receipt JSON as the payload.

Computing a unique transaction ID (SHA‑256 hash of the payload, a counter, and (optionally) the previous transaction hash).

Storing the Transaction object in a global in‑memory ledger.

On each node, transaction files are written to the folder:
$HOME/Ebeacon/transactions
Each file is named with the computed transaction hash (ending in .json). To view transactions, simply list or open these files.

File Summary
Files in the Root Directory (Ebeacon folder)
Cargo.toml / Cargo.lock:
Define project dependencies and lock versions.

deploy_tool.rs:
Deploys the event_beacon binary to remote nodes by stopping existing processes, copying files (binary & nodelist), setting permissions, and starting node mode via SSH.

stop_all_beacon.rs:
Connects via SSH to nodes (from the nodelist) to forcibly stop running event_beacon processes.

generate_keys.sh & generate_public_key.sh:
Shell scripts used to generate RSA keys.

nodelist:
Lists node names (one per line) used for deployment and stopping processes.

request_from_client.json:
A sample client request containing a signed JSON event request.

rpi.key & rpi.pub:
The private and public SSH keys used for secure remote connections.

Files in the src Folder
main.rs:
The main entry point for the event_beacon application.

action.rs:
Provides signing functions and nonce generation to produce signed BeaconStamp messages.

application.rs:
Coordinates the overall process: reads client request files, validates signatures, builds and broadcasts forward messages, collects BeaconStamp responses, builds the final receipt, and integrates with the Transaction layer.

handler.rs:
Implements HTTP endpoints (via Warp) for processing ClientRequest and Forward messages, including signature validation and logging.

host.rs:
Defines a Host struct that encapsulates the node’s unique identity (retrieved from the system hostname).

lib.rs:
Re-exports internal modules for library use.

message.rs:
Contains structured JSON definitions for ClientRequest, ForwardMessage (with its own signature), BeaconStamp, Receipt, etc.

network.rs:
Provides asynchronous functions to broadcast forward messages to nodes and collect BeaconStamp responses.

node.rs:
Implements signature validation functions for ClientRequest, BeaconStamp, and ForwardMessage.

record.rs:
Persists the complete JSON representation of transactions (e.g., signed BeaconStamp or Receipt) as files in $HOME/Ebeacon/transactions.

resource.rs:
Sets up HTTP endpoints for transaction processing.

role.rs:
Defines node roles (Coordinator, Participant); a stub for future extension.

service.rs:
Determines execution mode based on command-line arguments, handles remote executions (controller mode), and uses the Host module for node identity.

transaction.rs:
Defines the Transaction struct (with unique transaction hash, counter, and previous hash) and maintains a global in‑memory ledger.

tx_message.rs:
Defines the structure for raw transaction messages (if needed for deserialization).

tx_receipt.rs:
Defines the JSON structure for a transaction receipt (ACK/NACK).

user.rs:
Contains stub functions for client key validation.

utils.rs:
Provides utility functions (e.g., current timestamp, key loading).
