use std::collections::HashMap;

use sqlx::SqlitePool;
use tokio::sync::mpsc::{self, Receiver};
use tonic::transport::Channel;

use agent::job_client::JobClient;
use agent::{JobGuid, JobInfo, JobRequestResult, JobsList};

pub mod agent {
    tonic::include_proto!("agent");
}

use crate::agent_com::Agent;

#[derive(Debug)]
pub struct AgentState {
    job_client: Option<JobClient<Channel>>,
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

    async fn add(&mut self, guid: String) {
        if self.trackers.contains_key(&guid) {
            println!("Agent {} already existing", guid);
            return;
        }

        match Agent::get_by_guid(&guid, &self.db_pool).await {
            Ok(Some(agent)) => {
                let mut connection: Option<JobClient<Channel>> = None;
                match JobClient::connect(agent.endpoint).await {
                    Ok(conn) => {
                        connection = Some(conn);
                        println!("[AgentProcessor.add] agent {} successfully added", guid)
                    }
                    Err(err) => {
                        println!(
                            "[AgentProcessor.add] error communicating with agent {}",
                            guid
                        );
                    }
                }

                self.trackers.insert(
                    guid.clone(),
                    AgentState {
                        job_client: connection,
                    },
                );
            },
            Ok(None) => println!("[AgentProcessor.add] error agent {} doesn't exist", guid),
            Err(err) => println!("[AgentProcessor.add] error fetching agent: {}", guid),
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
            Some(agent) => {
                println!("[AgentProcessor.delete] agent {} successfully deleted", guid);
            },
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
