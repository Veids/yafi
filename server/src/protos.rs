#[allow(unused_imports)]
use agent::job_client::JobClient;

#[allow(unused_imports)]
use agent::system_info_client::SystemInfoClient;

#[allow(unused_imports)]
use agent::updates_client::UpdatesClient;

#[allow(unused_imports)]
use agent::{Empty, JobGuid, JobsList, SysInfo, Update};

#[allow(unused_imports)]
use agent::update::UpdateKind::CrashMsg;

pub mod agent {
    tonic::include_proto!("agent");
}
