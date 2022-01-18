use std::sync::{Arc, Mutex};

use sysinfo::{RefreshKind, System, SystemExt};
use tonic::{Request, Response, Status};

use agent::system_info_server::SystemInfo;
use agent::{Empty, SysInfo};

pub mod agent {
    tonic::include_proto!("agent");
}

#[derive(Debug)]
pub struct SystemInfoHandler {
    sys: Arc<Mutex<System>>,
}

impl SystemInfoHandler {
    pub fn new() -> SystemInfoHandler {
        SystemInfoHandler {
            sys: Arc::new(Mutex::new(System::new_with_specifics(
                RefreshKind::new().with_memory(),
            ))),
        }
    }
}

#[tonic::async_trait]
impl SystemInfo for SystemInfoHandler {
    async fn get(&self, _request: Request<Empty>) -> Result<Response<SysInfo>, Status> {
        let sys = self.sys.clone();
        let mut sys = sys.lock().unwrap();
        sys.refresh_memory();

        // There will be probably an overflow on 128 bit targets
        let reply = SysInfo {
            cpus: sys.physical_core_count().unwrap_or(0) as u64,
            ram: sys.total_memory(),
        };

        Ok(Response::new(reply))
    }
}
