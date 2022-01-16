use bollard::{
    container::{Config, CreateContainerOptions, StartContainerOptions, WaitContainerOptions},
    image::CreateImageOptions,
    Docker,
};
use futures::StreamExt;
use tokio::task;
use tonic::{transport::Server, Request, Response, Status};

use std::pin::Pin;
use std::time::Instant;

use dashmap::DashMap;
use std::sync::Arc;

use agent::job_server::{Job, JobServer};
use agent::{JobGuid, JobInfo, JobRequestResult, JobsList};

pub mod agent {
    tonic::include_proto!("agent");
}

#[derive(Debug)]
pub struct RuntimeInfo {
    state: String,
    started: Option<Instant>,
}

#[derive(Debug)]
pub struct JobInfoContainer {
    info: JobInfo, // JobInfo from create container request
    runtime_info: RuntimeInfo,
}

#[derive(Debug)]
pub struct JobHandler {
    docker: Arc<Docker>,
    jobs: Arc<DashMap<String, JobInfoContainer>>,
}

impl JobHandler {
    pub fn new(docker: Docker) -> JobHandler {
        JobHandler {
            docker: Arc::new(docker),
            jobs: Arc::new(DashMap::new()),
        }
    }
}

#[tonic::async_trait]
impl Job for JobHandler {
    async fn create(
        &self,
        request: Request<JobInfo>,
    ) -> Result<Response<JobRequestResult>, Status> {
        println!("Got a Job request: {:?}", request);

        let req = request.into_inner();

        let reply = agent::JobRequestResult {
            message: format!("Received request {}!", &req.guid).into(),
        };

        self.jobs.insert(
            req.guid.clone(),
            JobInfoContainer {
                info: req.clone(),
                runtime_info: RuntimeInfo {
                    state: "ImagePulling".to_string(),
                    started: None,
                },
            },
        );

        task::spawn({
            let jobs = Arc::clone(&self.jobs);
            let docker = Arc::clone(&self.docker);
            async move {
                let options = Some(CreateImageOptions {
                    from_image: req.image.clone(),
                    ..Default::default()
                });

                let mut stream = docker.create_image(options, None, None);
                loop {
                    match stream.next().await {
                        Some(state) => match state {
                            Ok(imageinfo) => match imageinfo.status {
                                Some(status) => {
                                    jobs.get_mut(&req.guid).unwrap().runtime_info.state =
                                        format!("Docker: {:?}", &status);
                                    println!("State {:?}", status);
                                }
                                None => {}
                            },
                            Err(err) => {
                                println!("Bollard_err {:?}", err);
                            }
                        },
                        None => {
                            println!("Received none");
                            break;
                        }
                    }
                }

                //TODO: check is image is really pulled
                let config = Config {
                    image: Some(req.image.clone()),
                    cmd: Some(vec!["echo".to_string(), "hello world".to_string()]),
                    ..Default::default()
                };

                match docker.create_container::<&str, String>(None, config).await {
                    Ok(res) => {
                        println!("Container created {:?}", res);

                        match docker.start_container::<String>(&res.id, None).await {
                            Ok(_) => {
                                println!("Container started");
                            }
                            Err(err) => {
                                println!("Bollard_err {:?}", err);
                            }
                        }

                        let mut stream = docker.wait_container(
                            &res.id,
                            Some(WaitContainerOptions {
                                condition: "not-running",
                            }),
                        );

                        loop {
                            match stream.next().await {
                                Some(response) => {
                                    println!("Container exited: {:?}", response);
                                }
                                None => break,
                            }
                        }

                        match docker.remove_container(&res.id, None).await {
                            Ok(_) => println!("Container removed"),
                            Err(err) => println!("Bollard_err {:?}", err),
                        }
                    }
                    Err(err) => {
                        println!("Bollard_err {:?}", err);
                    }
                }
            }
        });

        Ok(Response::new(reply))
    }

    async fn destroy(
        &self,
        request: Request<JobGuid>,
    ) -> Result<Response<JobRequestResult>, Status> {
        println!("Got a request: {:?}", request);

        let req = request.into_inner();
        let message;

        match self.jobs.remove(&req.guid) {
            Some(job) => {
                message = format!("Job {} has been destroyed", &req.guid);
            }
            None => {
                message = format!("Job.destroy: {} doesn't exist", &req.guid);
            }
        }

        println!("{}", message);
        let reply = agent::JobRequestResult { message: message };
        Ok(Response::new(reply))
    }

    async fn list(&self, request: Request<JobGuid>) -> Result<Response<JobsList>, Status> {
        let jobs = Arc::clone(&self.jobs);
        let reply = agent::JobsList {
            guids: jobs.iter().map(|k| k.key().clone()).collect(),
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
