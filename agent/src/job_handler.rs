use std::sync::Arc;
use std::path::Path;
use std::env;

use crate::jobs::Jobs;
use bollard::{
    container::{Config, WaitContainerOptions},
    models::HostConfig,
    image::CreateImageOptions,
    Docker,
    errors::Error as BollardError
};
use futures::StreamExt;
use tokio::sync::mpsc::Sender;
use tokio::sync::RwLock;
use tokio::task;
use tonic::{Request, Response, Status};

use crate::protos::agent::job_server::Job;
use crate::protos::agent::{
    update::UpdateKind, JobMsg, JobErr, JobStatus, Empty, JobCreateRequest, JobGuid,
    JobInfoContainerList, JobRequestResult, JobsList, Update,
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

struct JobItem {
    req: JobCreateRequest,
    docker: Arc<Docker>,
    jobs: Arc<Jobs>,
    updates: Arc<RwLock<Option<Sender<Update>>>>,
    id: Option<String>,
}

impl JobItem {
    pub fn new(req: JobCreateRequest, docker: Arc<Docker>, jobs: Arc<Jobs>, updates: Arc<RwLock<Option<Sender<Update>>>>) -> JobItem {
        JobItem {
            req: req,
            docker: docker,
            jobs: jobs,
            updates: updates,
            id: None
        }
    }

    async fn send_update(&self, kind: UpdateKind) {
        if let Some(tx) = &*self.updates.read().await {
            let _ = tx.send(Update {
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
                self.jobs.set_last_msg(
                    &self.req.job_guid,
                    status.to_string()
                );
                self.send_update(UpdateKind::JobMsg(JobMsg {
                    guid: self.req.job_guid.clone(),
                    last_msg: status.to_string()
                })).await;
            }
        }

        Ok(())
    }

    async fn create_container(&mut self) -> Result<(), BollardError> {
        let nfs_dir = env::var("NFS_DIR").expect("Set NFS_DIR in .env file");
        let job_dir = Path::new(&nfs_dir).join("jobs").join(&self.req.job_guid);
        let mount = vec!{
            format!("{}:/work", job_dir.into_os_string().into_string().unwrap())
        };
        let config = Config {
            image: Some(self.req.image.clone()),
            host_config: Some(HostConfig{
                binds: Some(mount),
                ..Default::default()
            }),
            cmd: Some(vec!["echo".to_string(), "hello world".to_string()]),
            ..Default::default()
        };


        self.id = Some(self.docker.create_container::<&str, String>(None, config).await?.id);
        Ok(())
    }

    async fn start_container(&self) -> Result<(), BollardError> {
        let res = self.docker.start_container::<String>(self.id.as_ref().unwrap(), None).await?;

        self.jobs.set_status(
            &self.req.job_guid,
            "alive"
        );

        self.send_update(UpdateKind::JobStatus(JobStatus{
            guid: self.req.job_guid.clone(),
            status: "alive".to_string()
        })).await;

        Ok(res)
    }

    async fn wait_container(&self) -> Result<(), BollardError> {
        let mut stream = self.docker.wait_container(
            &self.id.as_ref().unwrap(),
            Some(WaitContainerOptions {
                condition: "not-running",
            }),
        );

        while let Some(response) = stream.next().await {
            println!("Container exited: {:?}", response);
            self.send_update(UpdateKind::JobStatus(JobStatus{
                guid: self.req.job_guid.clone(),
                status: "completed".to_string()
            })).await;
            self.jobs.set_status(
                &self.req.job_guid,
                "completed"
            );
        }

        Ok(())
    }

    async fn remove_container(&mut self) -> Result<(), BollardError> {
        let res = self.docker.remove_container(&self.id.as_ref().unwrap(), None).await?;
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
            Ok(_) => {},
            Err(err) => {
                self.send_update(UpdateKind::JobErr(JobErr {
                    guid: self.req.job_guid.clone(),
                    last_msg: err.to_string()
                })).await;
            }
        }
    }
}

#[tonic::async_trait]
impl Job for JobHandler {
    async fn create(
        &self,
        request: Request<JobCreateRequest>,
    ) -> Result<Response<JobRequestResult>, Status> {
        println!("Got a Job request: {:?}", request);

        let req = request.into_inner();

        let reply = JobRequestResult {
            message: format!("Received request {}!", &req.job_guid).into(),
        };

        self.jobs.create(req.clone());

        task::spawn({
            let mut job_item = JobItem::new(req, self.docker.clone(), self.jobs.clone(), self.updates.clone());
            async move {
                job_item.main().await;
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
