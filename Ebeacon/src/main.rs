mod service;
mod application;
mod action;
mod handler;
mod resource;
mod transaction;
mod record;
mod node;
mod user;
mod role;
mod network;
mod host;
mod message;
mod utils;
mod tx_message;
mod tx_receipt;

#[tokio::main]
async fn main() {
    service::run_service().await;
}
