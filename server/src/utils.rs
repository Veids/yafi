use std::sync::Arc;

use log::error;
use tokio::sync::mpsc::Sender;

pub async fn notify_processor<T: std::fmt::Debug>(tx: &Arc<Sender<T>>, agent_update: T) {
    match tx.send(agent_update).await {
        Ok(_) => (),
        Err(err) => {
            error!("Failed to notify processor: {:#?}", err);
        }
    }
}
