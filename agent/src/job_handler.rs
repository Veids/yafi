use std::path::Path;
use std::sync::Arc;

use crate::config::CONFIG;
use crate::jobs::Jobs;
use bollard::{
    container::{Config, CreateContainerOptions, LogsOptions, WaitContainerOptions},
    errors::Error as BollardError,
    image::CreateImageOptions,
    models::HostConfig,
    Docker,
};
use futures::StreamExt;
use log::info;
use tokio::sync::mpsc::Sender;
use tokio::sync::RwLock;
use tokio::task;
use tonic::{Request, Response, Status};

use crate::protos::agent::job_server::Job;
use crate::protos::agent::{
    update::UpdateKind, Empty, JobCreateRequest, JobGuid, JobInfoContainerList, JobMsg, JobsList,
    Update,
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
            updates,
            docker: Arc::new(docker),
            jobs: Arc::new(Jobs::new()),
        }
    }
}

struct JobItem {
    req: JobCreateRequest,
    docker: Arc<Docker>,
    jobs: Arc<Jobs>,
    updates: Arc<RwLock<Option<Sender<Update>>>>,
    id: Option<String>,
}

impl JobItem {
    pub fn new(
        req: JobCreateRequest,
        docker: Arc<Docker>,
        jobs: Arc<Jobs>,
        updates: Arc<RwLock<Option<Sender<Update>>>>,
    ) -> JobItem {
        JobItem {
            req,
            docker,
            jobs,
            updates,
            id: None,
        }
    }

    async fn send_update(&self, kind: UpdateKind) {
        if let Some(tx) = &*self.updates.read().await {
            let _ = tx
                .send(Update {
                    update_kind: Some(kind),
                })
                .await;
        }
    }

    async fn pull_image(&self) -> Result<(), BollardError> {
        let options = Some(CreateImageOptions {
            from_image: self.req.image.clone(),
            ..Default::default()
        });

        let mut stream = self.docker.create_image(options, None, None);
        while let Some(state) = stream.next().await {
            let imageinfo = state?;
            if let Some(status) = imageinfo.status {
                self.jobs
                    .set_last_msg(&self.req.job_guid, status.to_string());
                self.send_update(UpdateKind::JobMsg(JobMsg {
                    guid: self.req.job_guid.clone(),
                    status: None,
                    last_msg: Some(status.to_string()),
                }))
                .await;
            }
        }

        Ok(())
    }

    async fn create_container(&mut self) -> Result<(), BollardError> {
        let job_dir = Path::new(&CONFIG.nfs_dir)
            .join("jobs")
            .join(&self.req.job_guid);
        let mount = vec![format!(
            "{}:/work",
            job_dir.into_os_string().into_string().unwrap()
        )];
        let config = Config {
            image: Some(self.req.image.clone()),
            host_config: Some(HostConfig {
                binds: Some(mount),
                ..Default::default()
            }),
            env: Some(vec![
                format!("ID={}", self.req.idx),
                format!("CPUS={}", self.req.cpus),
                "FUZZ_DIR=/root/fuzz".to_string(),
            ]),
            ..Default::default()
        };

        let options = Some(CreateContainerOptions {
            name: self.req.job_guid.clone(),
        });

        self.id = Some(self.docker.create_container(options, config).await?.id);
        Ok(())
    }

    async fn start_container(&self) -> Result<(), BollardError> {
        let res = self
            .docker
            .start_container::<String>(self.id.as_ref().unwrap(), None)
            .await?;

        self.jobs.set_status(&self.req.job_guid, "alive");

        self.send_update(UpdateKind::JobMsg(JobMsg {
            guid: self.req.job_guid.clone(),
            status: Some("alive".to_string()),
            last_msg: None,
        }))
        .await;

        Ok(res)
    }

    async fn wait_container(&self) -> Result<(), BollardError> {
        let mut stream = self.docker.wait_container(
            self.id.as_ref().unwrap(),
            Some(WaitContainerOptions {
                condition: "not-running",
            }),
        );

        while let Some(response) = stream.next().await {
            let response = response?;
            info!("Container exited: {:?}", response);

            if response.status_code == 0 {
                self.send_update(UpdateKind::JobMsg(JobMsg {
                    guid: self.req.job_guid.clone(),
                    status: Some("completed".to_string()),
                    last_msg: None,
                }))
                .await;
                self.jobs.set_status(&self.req.job_guid, "completed");
            } else {
                let mut log_stream = self.docker.logs::<String>(
                    self.id.as_ref().unwrap(),
                    Some(LogsOptions {
                        stderr: true,
                        ..Default::default()
                    }),
                );

                let mut log = String::new();

                while let Some(output) = log_stream.next().await {
                    let output = output?;
                    log += std::str::from_utf8(&output.into_bytes()).unwrap();
                }

                self.send_update(UpdateKind::JobMsg(JobMsg {
                    guid: self.req.job_guid.clone(),
                    status: Some("error".to_string()),
                    last_msg: Some(log),
                }))
                .await;
                self.jobs.set_status(&self.req.job_guid, "error");
            }
        }

        Ok(())
    }

    async fn remove_container(&mut self) -> Result<(), BollardError> {
        let res = self
            .docker
            .remove_container(self.id.as_ref().unwrap(), None)
            .await?;
        self.id = None;
        Ok(res)
    }

    async fn handle(&mut self) -> Result<(), BollardError> {
        self.pull_image().await?;
        self.create_container().await?;
        self.start_container().await?;
        self.wait_container().await?;
        self.remove_container().await
    }

    pub async fn main(&mut self) {
        match self.handle().await {
            Ok(_) => {}
            Err(err) => {
                self.send_update(UpdateKind::JobMsg(JobMsg {
                    guid: self.req.job_guid.clone(),
                    status: Some("error".to_string()),
                    last_msg: Some(err.to_string()),
                }))
                .await;
            }
        }
    }
}

#[tonic::async_trait]
impl Job for JobHandler {
    async fn create(&self, request: Request<JobCreateRequest>) -> Result<Response<Empty>, Status> {
        info!("Got a Job request: {:?}", request);

        let req = request.into_inner();
        self.jobs.create(req.clone());

        task::spawn({
            let mut job_item = JobItem::new(
                req,
                self.docker.clone(),
                self.jobs.clone(),
                self.updates.clone(),
            );
            async move {
                job_item.main().await;
            }
        });

        Ok(Response::new(Empty {}))
    }

    async fn destroy(&self, request: Request<JobGuid>) -> Result<Response<Empty>, Status> {
        info!("Got a request: {:?}", request);

        let req = request.into_inner();

        match self.jobs.destroy(&req.guid) {
            Some(_job) => Ok(Response::new(Empty {})),
            None => Err(Status::not_found(format!(
                "Job {} doesn't, exist",
                &req.guid
            ))),
        }
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

    async fn stop(&self, request: Request<JobGuid>) -> Result<Response<Empty>, Status> {
        let guid = request.into_inner().guid;

        if let Some(status) = self.jobs.get_status(&guid) {
            if status == "alive" || status == "init" {
                match self.docker.stop_container(&guid, None).await {
                    Ok(_) => return Ok(Response::new(Empty {})),
                    Err(msg) => return Err(Status::invalid_argument(msg.to_string())),
                }
            }
            return Err(Status::invalid_argument("Job has been finished"));
        }
        Err(Status::invalid_argument("Job not found"))
    }
}
