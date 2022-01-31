use futures::StreamExt;
use sqlx::SqlitePool;
use tokio::sync::mpsc::Receiver;
use tonic::transport::Channel;

use crate::protos::agent::job_client::JobClient;
use crate::protos::agent::system_info_client::SystemInfoClient;
use crate::protos::agent::updates_client::UpdatesClient;
use crate::protos::agent::{
    Empty, JobCreateRequest, JobGuid, JobRequestResult, JobsList, SysInfo, Update,
};

//TODO move to agent_db::AgentDb
use crate::agent_com::Agent;

#[derive(Debug)]
pub enum Request {
    JobCreate { job: JobCreateRequest },
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
            guid: guid,
            db_pool: db_pool,
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
                    Agent::update_status(&self.guid, "down", &self.db_pool)
                        .await
                        .unwrap();
                }
                Err(format!(
                    "[AgentBroker.init] error agent.JobClient {} couldn't establish connection",
                    self.guid
                ))?
            }

            if let Ok(conn) = UpdatesClient::connect(agent.endpoint.clone()).await {
                self.updates_client = Some(conn);
            } else {
                if agent.status != "up" {
                    Agent::update_status(&self.guid, "down", &self.db_pool)
                        .await
                        .unwrap();
                }
                Err(format!(
                    "[AgentBroker.init] error agent.UpdatesClient {} couldn't establish connection",
                    self.guid
                ))?
            }

            if let Ok(conn) = SystemInfoClient::connect(agent.endpoint).await {
                self.sys_info_client = Some(conn);
            } else {
                if agent.status != "up" {
                    Agent::update_status(&self.guid, "down", &self.db_pool)
                        .await
                        .unwrap();
                }
                Err(format!("[AgentBroker.init] error agent.SystemInfoClient {} coudln't establish connection", self.guid))?
            }

            if agent.status == "init" {
                if let Some(sys_info) = self.get_sysinfo().await {
                    match Agent::update_sys_info(&self.guid, &sys_info, &self.db_pool).await {
                        Ok(_) => {
                            Agent::update_status(&self.guid, "down", &self.db_pool)
                                .await
                                .unwrap();
                        }
                        Err(err) => Err(format!("Failed to update sys info: {err}"))?,
                    }
                }
            }

            Ok(())
        } else {
            Err(format!("[AgentBroker.init] agent {} not found", self.guid))
        }
    }

    async fn sync_jobs(&mut self) -> Result<(), String> {
        if let Some(job_client) = &mut self.job_client {
            let request = tonic::Request::new(Empty {});
            match job_client.get_all(request).await {
                Ok(response) => {
                    println!("job_client.get_all: {:?}", response.into_inner())
                }
                Err(_) => Err(format!(
                    "[AgentBroker.sync_jobs] failed to sync jobs with {}",
                    self.guid
                ))?,
            }
        } else {
            Err(format!(
                "[AgentBroker.sync_jobs] failed to get job_client for {}",
                self.guid
            ))?
        }

        Ok(())
    }

    async fn create_job(&mut self, job: JobCreateRequest) -> Result<(), String> {
        if let Some(job_client) = &mut self.job_client {
            let request = tonic::Request::new(job);
            match job_client.create(request).await {
                Ok(response) => {
                    println!("JobRequest successfully sent: {:?}", response.into_inner())
                }
                Err(err) => Err(format!(
                    "[AgentBroker.create_job] failed to create job: {:?}",
                    err
                ))?,
            }
        } else {
            Err(format!(
                "[AgentBroker.create_job] failed to get job_client for {:?}",
                self.guid
            ))?
        }

        Ok(())
    }

    async fn free_job_resources(&self, job_guid: &String) {
        match Agent::free_job_resources(&self.guid, &job_guid, &self.db_pool).await {
            Ok(_) => {}
            Err(err) => {
                println!(
                    "Failed to free {} job resources for {}: {:?}",
                    job_guid, self.guid, err
                );
            }
        }
    }

    async fn set_job_status(&self, job_guid: &String, status: &str) {
        match Agent::set_job_status(&self.guid, &job_guid, &status, &self.db_pool).await {
            Ok(_) => {}
            Err(err) => {
                println!(
                    "Failed to set {} job status for {}: {:?}",
                    job_guid, self.guid, err
                );
            }
        }
    }

    pub async fn main(&mut self, broker_messages: &mut Receiver<Request>) -> Result<(), String> {
        self.init().await?;
        self.sync_jobs().await?;

        let mut stream;
        match &mut self.updates_client {
            Some(updates_client) => {
                stream = updates_client.get(Empty {}).await.unwrap().into_inner();
            }
            _ => {
                return Err(format!(
                    "[AgentBroker.main] error agent.UpdatesClient {} is not ready",
                    self.guid
                ))
            }
        }

        Agent::update_status(&self.guid, "up", &self.db_pool)
            .await
            .unwrap();

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
                                        println!("{:?}", err);
                                        self.free_job_resources(&job_guid).await;
                                        self.set_job_status(&job_guid, "error").await;
                                    }
                                }
                            },
                        },
                        None => break
                    }
                },
                update = stream.next() => match update {
                    Some(update) => {},
                    None => break
                }
            }
        }

        Agent::update_status(&self.guid, "down", &self.db_pool)
            .await
            .unwrap();

        Ok(())
    }
}