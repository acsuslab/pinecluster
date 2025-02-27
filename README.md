Build the service:
cargo build --release --bin node
mv target/release/node target/release/start_service

Set permissions:
chmod 600 rpi.key
chmod 600 nodelist

Deploying Service:
cargo run --bin updater
