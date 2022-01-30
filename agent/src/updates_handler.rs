use std::sync::Arc;
use std::sync::RwLock;

use tokio::sync::mpsc::{self, Receiver, Sender};
// use tokio::sync::Mutex;
use tokio::task;
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Code, Request, Response, Status};

use crate::protos::agent::updates_server::Updates;
use crate::protos::agent::{Empty, JobInfo, JobInfoContainer, Update};

#[derive(Debug)]
pub struct UpdatesHandler {
    events_sender: Arc<RwLock<Option<Sender<Update>>>>,
}

impl UpdatesHandler {
    pub fn new(events: Arc<RwLock<Option<Sender<Update>>>>) -> UpdatesHandler {
        UpdatesHandler {
            events_sender: events,
        }
    }
}

#[tonic::async_trait]
impl Updates for UpdatesHandler {
    type GetStream = ReceiverStream<Result<Update, Status>>;

    async fn get(&self, _request: Request<Empty>) -> Result<Response<Self::GetStream>, Status> {
        let (tx, rx) = mpsc::channel(10);

        let (txr, mut rxr) = mpsc::channel::<Update>(100);
        {
            let mut res = self.events_sender.write().unwrap();
            if let Some(_) = *res {
                return Err(Status::new(
                    Code::Unavailable,
                    "Agent has been already connected",
                ));
            } else {
                *res = Some(txr);
            }
        }

        let events = self.events_sender.clone();
        task::spawn({
            async move {
                loop {
                    tokio::select! {
                        event = rxr.recv() => match event {
                            Some(event) => match tx.send(Ok(event.into())).await {
                                Ok(_) => (),
                                Err(_) => break
                            },
                            None => break
                        },
                        _ = tx.closed() => {
                            break;
                        }
                    }
                }
                println!("[UpdatesHandler] Server disconnected");
                let mut res = events.write().unwrap();
                *res = None;
            }
        });

        Ok(Response::new(ReceiverStream::new(rx)))
    }
}
