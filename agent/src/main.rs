use std::sync::Arc;

use bollard::Docker;
use dotenv::dotenv;
use log::info;
use tokio::sync::mpsc::Sender;
use tokio::sync::RwLock;
use tonic::transport::Server;

mod config;
use crate::config::CONFIG;

mod protos;
use protos::agent::job_server::JobServer;
use protos::agent::system_info_server::SystemInfoServer;
use protos::agent::updates_server::UpdatesServer;
use protos::agent::Update;

mod jobs;

mod job_handler;
use job_handler::JobHandler;

mod system_info;
use system_info::SystemInfoHandler;

mod updates_handler;
use updates_handler::UpdatesHandler;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();
    env_logger::init();

    let docker = Docker::connect_with_socket_defaults().unwrap();
    let tx: Arc<RwLock<Option<Sender<Update>>>> = Arc::new(RwLock::new(None));

    let addr = CONFIG.sap_agent_listen.parse()?;
    let job_handler = JobHandler::new(tx.clone(), docker);
    let system_info_handler = SystemInfoHandler::new();
    let updates_handler = UpdatesHandler::new(tx.clone());

    info!("Listening on {}", CONFIG.sap_agent_listen);

    Server::builder()
        .add_service(JobServer::new(job_handler))
        .add_service(SystemInfoServer::new(system_info_handler))
        .add_service(UpdatesServer::new(updates_handler))
        .serve(addr)
        .await?;

    Ok(())
}
