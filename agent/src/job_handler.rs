use std::{
    error, fs,
    path::{Path, PathBuf},
    pin::Pin,
    sync::Arc,
    time::Duration,
};

use crate::config::CONFIG;
use crate::jobs::Jobs;
use crate::protos::agent::CrashMsg;
use bollard::{
    container::{
        Config, CreateContainerOptions, InspectContainerOptions, LogsOptions, WaitContainerOptions,
    },
    errors::Error as BollardError,
    image::CreateImageOptions,
    models::{ContainerWaitResponse, HostConfig},
    Docker,
};
use futures::stream;
use futures_core::Stream;
use log::{error, info};
use tokio::{
    sync::{mpsc::Sender, RwLock},
    task, time,
};
use tokio_stream::StreamExt;
use tonic::{transport::Channel, Request, Response, Status};

use crate::protos::agent::job_server::Job;
use crate::protos::agent::{
    update::UpdateKind, AnalyzeRequest, AnalyzeResponse, Empty, JobCreateRequest, JobGuid,
    JobInfoContainerList, JobMsg, JobsList, Update,
};
use crate::protos::docker::process_client::ProcessClient;
use crate::protos::docker::CrashAnalyzeRequest;

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
    job_dir: PathBuf,
    docker_client: Option<ProcessClient<Channel>>,
}

impl JobItem {
    pub fn new(
        req: JobCreateRequest,
        docker: Arc<Docker>,
        jobs: Arc<Jobs>,
        updates: Arc<RwLock<Option<Sender<Update>>>>,
    ) -> JobItem {
        let job_dir = Path::new(&CONFIG.nfs_dir).join("jobs").join(&req.job_guid);

        JobItem {
            req,
            docker,
            jobs,
            updates,
            id: None,
            job_dir,
            docker_client: None,
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

    async fn get_logs(&self) -> Result<String, BollardError> {
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

        Ok(log)
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
                    log: None,
                }))
                .await;
            }
        }

        Ok(())
    }

    async fn create_container(&mut self) -> Result<(), BollardError> {
        let mount = vec![format!(
            "{}:/work",
            self.job_dir.to_string_lossy().into_owned()
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
            last_msg: Some("Container started".to_string()),
            log: None,
        }))
        .await;

        Ok(res)
    }

    async fn establish_connection(&mut self) -> Result<(), Box<dyn error::Error + Send + Sync>> {
        let options = Some(InspectContainerOptions { size: false });
        let response = self
            .docker
            .inspect_container(self.id.as_ref().unwrap(), options)
            .await?;

        let ip = response
            .network_settings
            .ok_or("Couldn't get container network settings")?
            .ip_address
            .ok_or("Couldn't get container ip address")?;

        info!("http://{ip}:50051");
        tokio::time::sleep(Duration::from_secs(5)).await;
        self.docker_client = Some(ProcessClient::connect(format!("http://{ip}:50051")).await?);

        Ok(())
    }

    async fn handle_response(&self, response: ContainerWaitResponse) -> Result<(), BollardError> {
        let logs = self.get_logs().await?;
        let status = if response.status_code == 0 {"completed"} else {"error"};
        self.send_update(UpdateKind::JobMsg(JobMsg{
            guid: self.req.job_guid.clone(),
            status: Some(status.to_string()),
            last_msg: Some("exited".to_string()),
            log: Some(logs),
        })).await;
        self.jobs.set_status(&self.req.job_guid, status);
        Ok(())
    }

    async fn analyze_crash(&mut self, file_name: String) -> Option<String> {
        match &mut self.docker_client {
            Some(conn) => {
                let request = tonic::Request::new(CrashAnalyzeRequest {
                    name: file_name.clone(),
                });
                match conn.analyze_crash(request).await {
                    Ok(res) => Some(res.into_inner().result),
                    Err(err) => {
                        info!("Failed to analyze crash {}: {}", file_name, err);
                        None
                    }
                }
            }
            None => {
                info!(
                    "Failed to analyze crash {}: connection is not ready",
                    file_name
                );
                None
            }
        }
    }

    async fn sync_crashes(&mut self) -> Result<(), BollardError> {
        let res_path = self.job_dir.join("res");
        let crashes_out = self.job_dir.join("crashes");

        for entry in fs::read_dir(res_path)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                let crashes_src = path.join("crashes");
                if let Ok(crashes) = fs::read_dir(crashes_src) {
                    for crash in crashes {
                        let crash = crash?;
                        let crash_path = crash.path();
                        let file_name = crash_path.file_name().unwrap_or_default();
                        let target = crashes_out.join(file_name);
                        if !target.exists() {
                            info!("New crash! {:?}", target);
                            let file_name = file_name.to_string_lossy().into_owned();
                            fs::copy(crash_path, target.clone())?;

                            let analyzed: Option<String> = if self.req.crash_auto_analyze {
                                self.analyze_crash(file_name.clone()).await
                            } else {
                                None
                            };

                            self.send_update(UpdateKind::CrashMsg(CrashMsg {
                                job_guid: self.req.job_guid.clone(),
                                name: file_name,
                                analyzed,
                            }))
                            .await;
                        }
                    }
                }
            }
        }

        Ok(())
    }

    async fn wait_container(&mut self) -> Result<(), BollardError> {
        let mut stream = self.docker.wait_container(
            self.id.as_ref().unwrap(),
            Some(WaitContainerOptions {
                condition: "not-running",
            }),
        );

        let mut sync_stream: Pin<Box<dyn Stream<Item = ()> + Send>> = if self.req.idx == 0 {
            let interval = time::interval(Duration::from_secs(60 * 5));
            Box::pin(stream::unfold(interval, |mut interval| async {
                interval.tick().await;
                Some(((), interval))
            }))
        } else {
            Box::pin(stream::empty())
        };

        loop {
            tokio::select! {
                Some(response) = stream.next() => {
                    let response = response?;
                    info!("Container exited: {:?}", response);

                    self.handle_response(response).await?;
                    break;
                },
                Some(_) = sync_stream.next() => {
                    self.sync_crashes().await?;
                },
                else => break
            }
        }

        self.sync_crashes().await?;

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

    async fn force_stop(&mut self) -> Result<(), BollardError> {
        self.docker
            .stop_container(self.id.as_ref().unwrap(), None)
            .await?;
        self.remove_container().await
    }

    async fn handle(&mut self) -> Result<(), Box<dyn error::Error + Send + Sync>> {
        self.pull_image().await?;
        self.create_container().await?;
        self.start_container().await?;
        self.establish_connection().await?;
        self.wait_container().await?;
        self.remove_container().await?;
        Ok(())
    }

    pub async fn main(&mut self) {
        match self.handle().await {
            Ok(_) => {}
            Err(err) => {
                error!("{:#?}", err);
                let logs = self.get_logs().await.ok();
                let err_msg = err.to_string();
                self.send_update(UpdateKind::JobMsg(JobMsg {
                    guid: self.req.job_guid.clone(),
                    status: Some("error".to_string()),
                    last_msg: Some(err_msg),
                    log: logs,
                }))
                .await;
                self.jobs.set_status(&self.req.job_guid, "error");

                _ = self.force_stop().await;
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

    async fn analyze_crash(
        &self,
        request: Request<AnalyzeRequest>,
    ) -> Result<Response<AnalyzeResponse>, Status> {
        Ok(Response::new(AnalyzeResponse {
            result: "".to_string(),
        }))
    }
}
