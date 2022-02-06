use std::sync::Arc;

use log::info;
use tokio::sync::mpsc::{self, Sender};
use tokio::sync::RwLock;
use tokio::task;
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Code, Request, Response, Status};

use crate::protos::agent::updates_server::Updates;
use crate::protos::agent::{Empty, Update};

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

    async fn get(&self, req: Request<Empty>) -> Result<Response<Self::GetStream>, Status> {
        let (tx, rx) = mpsc::channel(10);

        let (txr, mut rxr) = mpsc::channel::<Update>(100);
        {
            let mut res = self.events_sender.write().await;
            if res.is_some() {
                return Err(Status::new(
                    Code::Unavailable,
                    "Agent has been already connected",
                ));
            } else {
                *res = Some(txr);
            }
        }

        if let Some(addr) = req.remote_addr() {
            info!("[UpdatesHandler] Server {} connected!", addr);
        }

        let events = self.events_sender.clone();
        task::spawn({
            async move {
                loop {
                    tokio::select! {
                        event = rxr.recv() => match event {
                            Some(event) => match tx.send(Ok(event)).await {
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
                info!("[UpdatesHandler] Server disconnected");
                let mut _res = events.write().await;
                *_res = None;
            }
        });

        Ok(Response::new(ReceiverStream::new(rx)))
    }
}
