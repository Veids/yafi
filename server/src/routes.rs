use crate::handlers::{
    agent::{create, delete, get_all, get_by_guid},
    crash::get_crashes,
    job::{create_job, get_job, get_job_stats, get_jobs, stop_job},
    web::{agents, index, job, jobs},
};

use actix_files::Files;
use actix_web::web;

pub fn routes(cfg: &mut web::ServiceConfig) {
    cfg
        // API routes
        .service(
            web::scope("/api")
                // AGENT routes
                .service(get_all)
                .service(get_by_guid)
                .service(create)
                .service(delete)
                // JOB routes
                .service(create_job)
                .service(get_job_stats)
                .service(get_jobs)
                .service(get_job)
                .service(stop_job)
                // CRASH routes
                .service(get_crashes),
        )
        // WEB routes
        .service(index)
        .service(agents)
        .service(jobs)
        .service(job)
        .service(
            Files::new("/static", "./static/")
                .prefer_utf8(true)
                .index_file("static/html/404.html"),
        );
}
