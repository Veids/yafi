use log::error;

use std::collections::hash_map::Entry;
use std::collections::HashMap;

use sqlx::SqlitePool;
use tokio::{
    sync::mpsc::{self, Receiver, Sender},
    task,
};

use super::agent_broker::{AgentBroker, Request};

#[derive(Debug)]
pub enum Event {
    NewAgent { guid: String },
    DelAgent { guid: String },
    AgentRequest { guid: String, request: Request },
}

pub async fn broker(db_pool: SqlitePool, mut events: Receiver<Event>) {
    let (disconnect_sender, mut disconnect_receiver) =
        mpsc::channel::<(String, Receiver<Request>)>(100);
    let mut trackers: HashMap<String, Sender<Request>> = HashMap::new();
    loop {
        let event = tokio::select! {
            event = events.recv() => match event {
                None => break,
                Some(event) => event
            },
            disconnect = disconnect_receiver.recv() => {
                let (guid, _) = disconnect.unwrap();
                assert!(trackers.remove(&guid).is_some());
                continue;
            }
        };

        match event {
            Event::NewAgent { guid } => match trackers.entry(guid.clone()) {
                Entry::Occupied(_) => (),
                Entry::Vacant(entry) => {
                    let (client_sender, mut client_receiver) = mpsc::channel(100);
                    entry.insert(client_sender);
                    let guid = guid.clone();
                    let disconnect_sender = disconnect_sender.clone();
                    let db_pool = db_pool.clone();
                    task::spawn(async move {
                        {
                            let mut agent_broker = AgentBroker::new(guid.clone(), db_pool);
                            match agent_broker.main(&mut client_receiver).await {
                                Ok(_) => {}
                                Err(e) => error!("{}", e),
                            }
                        }
                        disconnect_sender
                            .send((guid, client_receiver))
                            .await
                            .unwrap();
                    });
                }
            },
            Event::DelAgent { guid } => {
                trackers.remove(&guid);
            }
            Event::AgentRequest { guid, request } => {
                if let Some(tracker) = trackers.get_mut(&guid) {
                    tracker.send(request).await.unwrap()
                }
            }
        }
    }
}
