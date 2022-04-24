use crate::handlers::{
    agent::{create, delete, get_all, get_by_guid},
    crash::{get_crash, get_crash_info, get_crash_stats, get_crashes},
    job::{create_job, get_job, get_job_crashes, get_job_stats, get_jobs, stop_job},
    stats::{query_job_stats, query_stats},
    web::{agents, crash, crashes, index, job, jobs},
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
                .service(get_job_crashes)
                // CRASH routes
                .service(get_crashes)
                .service(get_crash_stats)
                .service(get_crash_info)
                .service(get_crash)
                // SATS routes
                .service(query_stats)
                .service(query_job_stats),
        )
        // WEB routes
        .service(index)
        .service(agents)
        .service(jobs)
        .service(job)
        .service(crashes)
        .service(crash)
        .service(
            Files::new("/static", "./static/")
                .prefer_utf8(true)
                .index_file("static/html/404.html"),
        );
}
