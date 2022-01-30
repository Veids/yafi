use agent::job_server::Job;
use agent::{Empty, JobGuid, JobInfo, JobInfoContainer, JobRequestResult, JobsList};

pub mod agent {
    tonic::include_proto!("agent");
}
