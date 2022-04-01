#[allow(unused_imports)]
use agent::job_server::Job;
#[allow(unused_imports)]
use agent::{
    update::UpdateKind::JobMsg, AnalyzeRequest, AnalyzeResponse, Empty, JobCreateRequest, JobGuid,
    JobsList,
};
#[allow(unused_imports)]
use docker::{CrashAnalyzeRequest, CrashAnalyzeResponse};

pub mod agent {
    tonic::include_proto!("agent");
}

pub mod docker {
    tonic::include_proto!("docker");
}
