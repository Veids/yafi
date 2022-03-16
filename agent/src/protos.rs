#[allow(unused_imports)]
use agent::job_server::Job;
#[allow(unused_imports)]
use agent::{
    update::UpdateKind::JobMsg, CrashAnalyzeRequest, Empty, JobCreateRequest, JobGuid, JobsList,
};

pub mod agent {
    tonic::include_proto!("agent");
}
