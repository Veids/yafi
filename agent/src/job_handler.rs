use std::sync::{Arc, RwLock};
use std::time::Instant;

use crate::jobs::Jobs;
use bollard::{
    container::{Config, WaitContainerOptions},
    image::CreateImageOptions,
    Docker,
};
use dashmap::DashMap;
use futures::StreamExt;
use tokio::sync::mpsc::Sender;
use tokio::task;
use tonic::{Request, Response, Status};

use crate::protos::agent::job_server::Job;
use crate::protos::agent::{
    Empty, JobGuid, JobInfo, JobInfoContainer, JobInfoContainerList, JobRequestResult,
    JobRuntimeInfo, JobsList, Update,
};

#[derive(Debug)]
pub struct JobHandler {
    updates: Arc<RwLock<Option<Sender<Update>>>>,
    docker: Arc<Docker>,
    jobs: Arc<Jobs>,
}

impl JobHandler {
    pub fn new(updates: Arc<RwLock<Option<Sender<Update>>>>, docker: Docker) -> JobHandler {
        JobHandler {
            updates: updates,
            docker: Arc::new(docker),
            jobs: Arc::new(Jobs::new()),
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

        let reply = JobRequestResult {
            message: format!("Received request {}!", &req.guid).into(),
        };

        self.jobs.create(req.clone());

        task::spawn({
            let jobs = self.jobs.clone();
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
                                    jobs.update_status(&req.guid, format!("Docker: {:?}", status));
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

        match self.jobs.destroy(&req.guid) {
            Some(_job) => {
                message = format!("Job {} has been destroyed", &req.guid);
            }
            None => {
                message = format!("Job.destroy: {} doesn't exist", &req.guid);
            }
        }

        println!("{}", message);
        let reply = JobRequestResult { message: message };
        Ok(Response::new(reply))
    }

    async fn list(&self, _request: Request<Empty>) -> Result<Response<JobsList>, Status> {
        let reply = JobsList {
            guids: self.jobs.guids(),
        };

        Ok(Response::new(reply))
    }

    async fn get_all(&self, _: Request<Empty>) -> Result<Response<JobInfoContainerList>, Status> {
        Ok(Response::new(JobInfoContainerList {
            jobs: self.jobs.get_all(),
        }))
    }
}
