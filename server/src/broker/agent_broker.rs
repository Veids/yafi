use futures::StreamExt;
use log::error;
use sqlx::SqlitePool;
use tokio::sync::mpsc::Receiver;
use tonic::transport::Channel;

use crate::protos::agent::job_client::JobClient;
use crate::protos::agent::system_info_client::SystemInfoClient;
use crate::protos::agent::updates_client::UpdatesClient;
use crate::protos::agent::JobGuid;
use crate::protos::agent::{update::UpdateKind, Empty, JobCreateRequest, SysInfo};

use crate::models::Agent;
use crate::models::Job;

#[derive(Debug)]
pub enum Request {
    JobCreate { job: JobCreateRequest },
    JobStop { guid: String },
}

#[derive(Debug)]
pub struct AgentBroker {
    guid: String,
    db_pool: SqlitePool,
    job_client: Option<JobClient<Channel>>,
    updates_client: Option<UpdatesClient<Channel>>,
    sys_info_client: Option<SystemInfoClient<Channel>>,
}

impl AgentBroker {
    pub fn new(guid: String, db_pool: SqlitePool) -> AgentBroker {
        AgentBroker {
            guid,
            db_pool,
            job_client: None,
            updates_client: None,
            sys_info_client: None,
        }
    }

    async fn get_sysinfo(&mut self) -> Option<SysInfo> {
        if let Some(sys_info_client) = &mut self.sys_info_client {
            let request = tonic::Request::new(Empty {});
            match sys_info_client.get(request).await {
                Ok(response) => Some(response.into_inner()),
                Err(_) => None,
            }
        } else {
            None
        }
    }

    async fn init(&mut self) -> Result<(), String> {
        if let Ok(Some(agent)) = Agent::get_by_guid(&self.guid, &self.db_pool).await {
            if let Ok(conn) = JobClient::connect(agent.endpoint.clone()).await {
                self.job_client = Some(conn);
            } else {
                if agent.status == "up" {
                    self.update_status("down").await;
                }
                return Err(format!(
                    "agent.JobClient {} couldn't establish connection",
                    self.guid
                ));
            }

            if let Ok(conn) = UpdatesClient::connect(agent.endpoint.clone()).await {
                self.updates_client = Some(conn);
            } else {
                if agent.status != "up" {
                    self.update_status("down").await;
                }
                return Err(format!(
                    "agent.UpdatesClient {} couldn't establish connection",
                    self.guid
                ));
            }

            if let Ok(conn) = SystemInfoClient::connect(agent.endpoint).await {
                self.sys_info_client = Some(conn);
            } else {
                if agent.status != "up" {
                    self.update_status("down").await;
                }
                return Err(format!(
                    "agent.SystemInfoClient {} coudln't establish connection",
                    self.guid
                ));
            }

            if agent.status == "init" {
                if let Some(sys_info) = self.get_sysinfo().await {
                    match Agent::update_sys_info(&self.guid, &sys_info, &self.db_pool).await {
                        Ok(_) => self.update_status("down").await,
                        Err(err) => return Err(format!("Failed to update sys info: {err}")),
                    }
                }
            }

            Ok(())
        } else {
            Err(format!("Agent {} not found", self.guid))
        }
    }

    async fn sync_jobs(&mut self) -> Result<(), String> {
        if let Some(job_client) = &mut self.job_client {
            let request = tonic::Request::new(Empty {});
            match job_client.get_all(request).await {
                Ok(response) => {
                    match Job::sync_jobs(&self.guid, response.into_inner(), &self.db_pool).await {
                        Ok(_) => {}
                        Err(err) => {
                            return Err(format!(
                                "failed to sync jobs with {}: {:?}",
                                self.guid, err
                            ))
                        }
                    }
                }
                Err(err) => {
                    return Err(format!("failed to sync jobs with {}: {:?}", self.guid, err))
                }
            }
        } else {
            return Err(format!("failed to get job_client for {}", self.guid));
        }

        Ok(())
    }

    async fn create_job(&mut self, job: JobCreateRequest) -> Result<(), String> {
        if let Some(job_client) = &mut self.job_client {
            let request = tonic::Request::new(job);
            match job_client.create(request).await {
                Ok(_) => {}
                Err(err) => return Err(format!("failed to create job: {:?}", err)),
            }
        } else {
            return Err(format!("failed to get job_client for {:?}", self.guid));
        }

        Ok(())
    }

    async fn set_job_status(&self, job_guid: &str, status: &str) {
        match Job::set_job_status(&self.guid, job_guid, status, &self.db_pool).await {
            Ok(_) => {}
            Err(err) => {
                error!(
                    "Failed to set {} job status for {}: {:?}",
                    job_guid, self.guid, err
                );
            }
        }
    }

    async fn set_job_last_msg(&self, job_guid: &str, last_msg: &str) {
        match Job::set_job_last_msg(&self.guid, job_guid, last_msg, &self.db_pool).await {
            Ok(_) => {}
            Err(err) => {
                error!(
                    "Failed to set {} job last_msg for {}: {:?}",
                    job_guid, self.guid, err
                );
            }
        }
    }

    async fn complete_job(&self, job_guid: &str, last_msg: &str, status: &str) {
        match Job::complete_job(&self.guid, job_guid, last_msg, status, &self.db_pool).await {
            Ok(_) => {}
            Err(err) => {
                error!(
                    "Failed to complete {} job for {}: {:?}",
                    job_guid, self.guid, err
                );
            }
        }
    }

    async fn stop_job(&mut self, job_guid: &str) -> Result<(), String> {
        if let Some(job_client) = &mut self.job_client {
            let request = tonic::Request::new(JobGuid {
                guid: job_guid.to_string(),
            });
            match job_client.stop(request).await {
                Ok(_) => {}
                Err(err) => return Err(format!("failed to stop job: {:?}", err)),
            }
        } else {
            return Err(format!("failed to get job_client for {:?}", self.guid));
        }

        Ok(())
    }

    async fn update_status(&self, status: &str) {
        Agent::update_status(&self.guid, status, &self.db_pool)
            .await
            .unwrap();
    }

    pub async fn main(&mut self, broker_messages: &mut Receiver<Request>) -> Result<(), String> {
        self.init().await?;
        self.sync_jobs().await?;

        let mut stream = match &mut self.updates_client {
            Some(updates_client) => updates_client.get(Empty {}).await.unwrap().into_inner(),
            _ => return Err(format!("agent.UpdatesClient {} is not ready", self.guid)),
        };

        self.update_status("up").await;

        loop {
            tokio::select! {
                msg = broker_messages.recv() => {
                    match msg {
                        Some(msg) => match msg {
                            Request::JobCreate { job } => {
                                let job_guid = job.job_guid.clone();
                                match self.create_job(job).await {
                                    Ok(_) => {},
                                    Err(err) => {
                                        self.complete_job(&job_guid, &err.to_string(), "error").await;
                                    }
                                }
                            },
                            Request::JobStop { guid } => {
                                match self.stop_job(&guid).await {
                                    Ok(_) => {},
                                    Err(err) => {
                                        error!("{:?}", err);
                                    }
                                }
                            },
                        },
                        None => break
                    }
                },
                update = stream.next() => match update {
                    Some(update) => {
                        match update {
                            Ok(update) => {
                                if let Some(kind) = update.update_kind {
                                    match kind {
                                        UpdateKind::JobMsg(job_update) => {
                                            if let Some(status) = job_update.status {
                                                let last_msg = job_update.last_msg.unwrap_or_default();
                                                if status == "completed" || status == "error" {
                                                    self.complete_job(&job_update.guid, &last_msg, &status).await;
                                                } else {
                                                    self.set_job_status(&job_update.guid, &status).await;
                                                }
                                            } else if let Some(last_msg) = job_update.last_msg {
                                                self.set_job_last_msg(&job_update.guid, &last_msg).await;
                                            }
                                        },
                                    }
                                }
                            },
                            Err(err) => {
                                self.update_status("down").await;
                                return Err(format!(
                                    "agent.UpdatesClient {} throwed an error: {:?}",
                                    self.guid, err
                                ))
                            }
                        }
                    },
                    None => break
                }
            }
        }

        self.update_status("down").await;

        Ok(())
    }
}
