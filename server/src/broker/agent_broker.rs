use futures::StreamExt;
use sqlx::SqlitePool;
use tokio::sync::mpsc::Receiver;
use tonic::transport::Channel;

use crate::protos::agent::job_client::JobClient;
use crate::protos::agent::system_info_client::SystemInfoClient;
use crate::protos::agent::updates_client::UpdatesClient;
use crate::protos::agent::{Empty, JobGuid, JobInfo, JobRequestResult, JobsList, SysInfo, Update};

//TODO move to agent_db::AgentDb
use crate::agent_com::Agent;

#[derive(Debug)]
pub enum Request {
    JobCreate {},
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

    pub async fn main(&mut self, broker_messages: &mut Receiver<Request>) -> Result<(), String> {
        self.init().await?;

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
                            Request::JobCreate { } => {},
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
