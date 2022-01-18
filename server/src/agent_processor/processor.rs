use std::collections::hash_map::Entry::Vacant;
use std::collections::HashMap;

use sqlx::SqlitePool;
use tokio::sync::mpsc::Receiver;
use tonic::transport::Channel;

use agent::job_client::JobClient;
use agent::system_info_client::SystemInfoClient;
use agent::{Empty, JobGuid, JobInfo, JobRequestResult, JobsList, SysInfo};

pub mod agent {
    tonic::include_proto!("agent");
}

use crate::agent_com::Agent;

#[derive(Debug)]
pub struct AgentState {
    job_client: Option<JobClient<Channel>>,
    sys_info_client: Option<SystemInfoClient<Channel>>,
}

#[derive(Debug)]
pub struct AgentUpdate {
    pub guid: String,
    pub update_type: String,
}

#[derive(Debug)]
pub struct AgentProcessor {
    trackers: HashMap<String, AgentState>,
    agent_updates: Receiver<AgentUpdate>,
    db_pool: SqlitePool,
}

impl AgentProcessor {
    pub fn new(agent_updates: Receiver<AgentUpdate>, db_pool: SqlitePool) -> AgentProcessor {
        AgentProcessor {
            trackers: HashMap::new(),
            agent_updates: agent_updates,
            db_pool: db_pool,
        }
    }

    async fn get_sysinfo(&mut self, guid: String) -> Option<SysInfo> {
        match self.trackers.get_mut(&guid) {
            Some(agent_state) => {
                let request = tonic::Request::new(Empty {});
                if let Some(sys_info_client) = &mut agent_state.sys_info_client {
                    match sys_info_client.get(request).await {
                        Ok(response) => Some(response.into_inner()),
                        Err(err) => {
                            println!("error executing SysInfoClient.get: {}", err);
                            None
                        }
                    }
                } else {
                    println!("bad channel");
                    None
                }
            }
            None => {
                println!(
                    "[AgentProcessor.get_sysinfo] error agent {} doesn't exist",
                    guid
                );
                None
            }
        }
    }

    async fn add(&mut self, guid: String) {
        if self.trackers.contains_key(&guid) {
            println!("Agent {} already existing", guid);
            return;
        }

        match Agent::get_by_guid(&guid, &self.db_pool).await {
            Ok(Some(agent)) => {
                match self.trackers.entry(guid.clone()) {
                    Vacant(entry) => {
                        let mut agent_state = AgentState {
                            job_client: None,
                            sys_info_client: None,
                        };

                        match JobClient::connect(agent.endpoint.clone()).await {
                            Ok(conn) => {
                                agent_state.job_client = Some(conn);
                                println!(
                                    "[AgentProcessor.add] agent.JobClient {} successfully added",
                                    guid
                                )
                            }
                            Err(err) => {
                                println!(
                                    "[AgentProcessor.add] error communicating with agent {}: {}",
                                    guid, err
                                );
                            }
                        }

                        match SystemInfoClient::connect(agent.endpoint).await {
                            Ok(conn) => {
                                agent_state.sys_info_client = Some(conn);
                                println!("[AgentProcessor.add] agent.SystemInfoClient {} successfully added", guid)
                            }
                            Err(err) => {
                                println!(
                                    "[AgentProcessor.add] error communicating with agent {}: {}",
                                    guid, err
                                );
                            }
                        }

                        entry.insert(agent_state);
                    }
                    _ => (),
                }

                if let Some(sys_info) = self.get_sysinfo(guid.clone()).await {
                    match Agent::update_sys_info(&guid, &sys_info, &self.db_pool).await {
                        Ok(_) => println!("[AgentProcessor.add] Succesfully updated sys info"),
                        Err(err) => println!("Failed to update sys info: {}", err),
                    }
                }
            }
            Ok(None) => println!("[AgentProcessor.add] error agent {} doesn't exist", guid),
            Err(err) => println!(
                "[AgentProcessor.add] error fetching agent {}: {}",
                guid, err
            ),
        }
    }

    async fn add_existing(&mut self) {
        match Agent::get_all(&self.db_pool).await {
            Ok(agents) => {
                for agent in agents {
                    self.add(agent.guid).await;
                }
            }
            Err(err) => {
                println!(
                    "[AgentProcessor.add_existing] error fetching agents: {}",
                    err
                );
            }
        }
    }

    async fn del(&mut self, guid: String) {
        match self.trackers.remove(&guid) {
            Some(_agent) => {
                println!(
                    "[AgentProcessor.delete] agent {} successfully deleted",
                    guid
                );
            }
            None => {
                println!("[AgentProcessor.delete] error agent {} doesnt' exist", guid);
            }
        }
    }

    pub async fn main(&mut self) {
        self.add_existing().await;

        loop {
            match self.agent_updates.recv().await {
                Some(x) => match x.update_type.as_ref() {
                    "add" => self.add(x.guid).await,
                    "del" => self.del(x.guid).await,
                    _ => println!("Unkown update type: {}", x.update_type),
                },
                None => break,
            }
        }
    }
}
