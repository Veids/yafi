use crate::protos::agent::JobCreateRequest;
use dashmap::DashMap;

#[derive(Debug)]
pub struct Jobs {
    jobs: DashMap<String, JobCreateRequest>,
}

impl Jobs {
    pub fn new() -> Jobs {
        Jobs {
            jobs: DashMap::new(),
        }
    }

    pub fn get_all(&self) -> Vec<JobCreateRequest> {
        self.jobs.iter().map(|k| k.value().clone()).collect()
    }

    pub fn create(&self, req: JobCreateRequest) {
        self.jobs.insert(
            req.job_guid.clone(),
            req
        );
    }

    pub fn set_status(&self, guid: &String, status: &str) {
        if let Some(mut rt) = self.jobs.get_mut(guid) {
            rt.status = status.to_string();
        }
    }

    pub fn set_last_msg(&self, guid: &String, message: String) {
        if let Some(mut rt) = self.jobs.get_mut(guid) {
            rt.last_msg = message;
        }
    }

    pub fn destroy(&self, guid: &String) -> Option<(String, JobCreateRequest)> {
        self.jobs.remove(guid)
    }

    pub fn guids(&self) -> Vec<String> {
        self.jobs.iter().map(|k| k.key().clone()).collect()
    }
}
