#[allow(unused_imports)]
use agent::job_server::Job;
#[allow(unused_imports)]
use agent::{
    update::Kind::JobUpdate, Empty, JobCreateRequest, JobGuid, JobInfoContainer, JobRequestResult,
    JobsList,
};

pub mod agent {
    tonic::include_proto!("agent");
}
