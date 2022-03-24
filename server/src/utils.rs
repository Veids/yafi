use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use log::error;
use tokio::sync::mpsc::Sender;

use crate::config::CONFIG;

pub async fn notify_processor<T: std::fmt::Debug>(tx: &Arc<Sender<T>>, agent_update: T) {
    match tx.send(agent_update).await {
        Ok(_) => (),
        Err(err) => {
            error!("Failed to notify processor: {:#?}", err);
        }
    }
}

pub fn get_job_dir(job_guid: &str) -> PathBuf {
    Path::new(&CONFIG.nfs_dir).join("jobs").join(job_guid)
}
