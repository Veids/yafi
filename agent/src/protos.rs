#[allow(unused_imports)]
use agent::job_server::Job;
#[allow(unused_imports)]
use agent::{
    update::UpdateKind::JobMsg, Empty, JobCreateRequest, JobGuid, JobRequestResult, JobsList,
};

pub mod agent {
    tonic::include_proto!("agent");
}
