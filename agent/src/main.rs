use tonic::{transport::Server, Request, Response, Status};
use bollard::Docker;
use bollard::image::ListImagesOptions;
use bollard::container::{CreateContainerOptions, Config};
use tokio::task;

use std::time::{Instant};

use std::collections::HashMap;

use agent::job_server::{Job, JobServer};
use agent::{JobInfo, JobRequestResult, JobGuid, JobsList};

pub mod agent {
    tonic::include_proto!("agent");
}

#[derive(Debug)]
pub struct RuntimeInfo {
    state: String,
    started: Option<Instant>
}

#[derive(Debug)]
pub struct JobInfoContainer {
    info: JobInfo, // JobInfo from create container request
    runtime_info: RuntimeInfo
}

#[derive(Debug)]
pub struct JobHandler {
    docker: Docker,
    jobs: HashMap<String, JobInfoContainer>
}

impl JobHandler {
    pub fn new(docker: Docker) -> JobHandler {
        JobHandler {
            docker: docker,
            jobs: HashMap::new()
        }
    }
}

#[tonic::async_trait]
impl Job for JobHandler {
    async fn create(
        &self,
        request: Request<JobInfo>
    ) -> Result<Response<JobRequestResult>, Status> {
        println!("Got a Job request: {:?}", request);

        let req = request.into_inner();

        let options = Some(CreateContainerOptions{
            name: &req.guid
        });

        let config = Config {
            image: Some(req.image),
            cmd: Some(vec!["bash".to_string(), "echo".to_string(), "hello".to_string()]),
            ..Default::default()
        };

        task::spawn(async {
            println!("Hello world!");
        });

        let reply = agent::JobRequestResult {
            message: format!("Received request {}!", &req.guid).into(),
        };

        &self.jobs.insert(
            req.guid,
            JobInfoContainer {
                info: req,
                runtime_info: RuntimeInfo {
                    state: "ImagePulling".to_string(),
                    started: None
                }
            }
        );

        Ok(Response::new(reply))
    }

    async fn destroy(
        &self,
        request: Request<JobGuid>
    ) -> Result<Response<JobRequestResult>, Status> {
        println!("Got a request: {:?}", request);

        let reply = agent::JobRequestResult {
            message: format!("Job {} has been destroyed!", request.into_inner().guid).into(),
        };

        Ok(Response::new(reply))
    }

    async fn list(
        &self,
        request: Request<JobGuid>
    ) -> Result<Response<JobsList>, Status> {
        let reply = agent::JobsList {
            guids: vec![ agent::JobGuid{ guid: "aloha".into() }]
        };

        Ok(Response::new(reply))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let docker = Docker::connect_with_socket_defaults().unwrap();

    let addr = "[::1]:50051".parse()?;
    let job_server = JobHandler::new(docker);

    Server::builder()
        .add_service(JobServer::new(job_server))
        .serve(addr)
        .await?;

    Ok(())
}
