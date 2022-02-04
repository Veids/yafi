use crate::server::server;

mod broker;
mod config;
mod handlers;
mod models;
mod protos;
mod routes;
mod server;
mod utils;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    server().await
}
