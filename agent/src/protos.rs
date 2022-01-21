use agent::job_server::Job;
use agent::{Empty, JobGuid, JobRequestResult, JobsList, JobInfoContainer, JobInfo};

pub mod agent {
    tonic::include_proto!("agent");
}
