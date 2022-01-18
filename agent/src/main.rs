use bollard::Docker;
use tonic::transport::Server;

mod job_handler;
use job_handler::agent::job_server::JobServer;
use job_handler::JobHandler;

mod system_info;
use system_info::agent::system_info_server::SystemInfoServer;
use system_info::SystemInfoHandler;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let docker = Docker::connect_with_socket_defaults().unwrap();

    let addr = "[::1]:50051".parse()?;
    let job_handler = JobHandler::new(docker);
    let system_info_handler = SystemInfoHandler::new();

    Server::builder()
        .add_service(JobServer::new(job_handler))
        .add_service(SystemInfoServer::new(system_info_handler))
        .serve(addr)
        .await?;

    Ok(())
}
