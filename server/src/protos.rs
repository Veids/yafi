use agent::job_client::JobClient;
use agent::system_info_client::SystemInfoClient;
use agent::updates_client::UpdatesClient;
use agent::{Empty, JobGuid, JobInfo, JobRequestResult, JobsList, SysInfo, Update};

pub mod agent {
    tonic::include_proto!("agent");
}
