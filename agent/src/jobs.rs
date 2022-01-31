use crate::protos::agent::{JobCreateRequest, JobInfoContainer, JobRuntimeInfo};
use dashmap::DashMap;

#[derive(Debug)]
pub struct Jobs {
    jobs: DashMap<String, JobInfoContainer>,
}

impl Jobs {
    pub fn new() -> Jobs {
        Jobs {
            jobs: DashMap::new(),
        }
    }

    pub fn get_all(&self) -> Vec<JobInfoContainer> {
        self.jobs.iter().map(|k| k.value().clone()).collect()
    }

    pub fn create(&self, req: JobCreateRequest) {
        self.jobs.insert(
            req.job_guid.clone(),
            JobInfoContainer {
                info: Some(req),
                runtime_info: Some(JobRuntimeInfo {
                    status_msg: "init".to_string(),
                }),
            },
        );
    }

    pub fn update_status(&self, guid: &String, message: String) {
        if let Some(rt) = &mut (*self.jobs.get_mut(guid).unwrap()).runtime_info {
            rt.status_msg = message;
        }
    }

    pub fn destroy(&self, guid: &String) -> Option<(String, JobInfoContainer)> {
        self.jobs.remove(guid)
    }

    pub fn guids(&self) -> Vec<String> {
        self.jobs.iter().map(|k| k.key().clone()).collect()
    }
}
