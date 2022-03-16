use std::io::prelude::*;
use std::{fs, path::Path};

use crate::broker::{Event, Request};
use crate::config::CONFIG;
use crate::handlers::agent::JobInfo;
use crate::models::Job;
use crate::utils::notify_processor;

use actix_multipart::{Field, Multipart};
use actix_web::{get, post, web, Error, HttpResponse, Responder};
use futures::StreamExt;
use log::{error, info};
use sqlx::SqlitePool;
use tokio::sync::mpsc::Sender;
use uuid::Uuid;

async fn fetch_file(mut field: Field, path: &Path) -> Result<(), Error> {
    let mut target = fs::File::create(path)?;
    while let Some(chunk) = field.next().await {
        target.write_all(&chunk?)?;
    }
    Ok(())
}

async fn process_job_create(payload: &mut Multipart, job_dir: &Path) -> Result<JobInfo, Error> {
    let mut job_info = JobInfo {
        ..Default::default()
    };
    while let Some(item) = payload.next().await {
        let mut field = item?;

        let name = field.content_disposition().get_name().unwrap().to_string();
        match name.as_ref() {
            "target" => {
                fetch_file(field, &job_dir.join("target.zip")).await?;
                job_info.target = "target.zip".to_string();
            }
            "corpus" => {
                fetch_file(field, &job_dir.join("corpus.zip")).await?;
                job_info.corpus = "corpus.zip".to_string();
            }
            _ => {
                let chunk = field.next().await.unwrap()?;
                match name.as_ref() {
                    "name" => {
                        job_info.name = std::str::from_utf8(&chunk).unwrap_or("").to_string();
                    }
                    "description" => {
                        job_info.description = std::str::from_utf8(&chunk).unwrap().to_string();
                    }
                    "agent-type" => {
                        job_info.agent_type = std::str::from_utf8(&chunk).unwrap_or("").to_string();
                    }
                    "image" => {
                        job_info.image = std::str::from_utf8(&chunk).unwrap_or("").to_string();
                    }
                    "cpus" => {
                        job_info.cpus =
                            std::str::from_utf8(&chunk).unwrap().parse::<u64>().unwrap();
                    }
                    "ram" => {
                        job_info.ram = std::str::from_utf8(&chunk).unwrap().parse::<u64>().unwrap();
                    }
                    "timeout" => {
                        job_info.timeout = std::str::from_utf8(&chunk).unwrap().to_string();
                    }
                    _ => {}
                }
            }
        }
    }
    Ok(job_info)
}

fn sanitize_job_info(job_info: &JobInfo) -> Result<(), Error> {
    if job_info.agent_type == "linux" && job_info.image.is_empty() {
        return Err(actix_web::error::ErrorBadRequest(
            "you haven't specified image",
        ));
    }

    if job_info.cpus == 0 {
        return Err(actix_web::error::ErrorBadRequest(
            "you haven't number of cpu cores",
        ));
    }

    if job_info.timeout.is_empty() && job_info.timeout.parse::<humantime::Duration>().is_err() {
        return Err(actix_web::error::ErrorBadRequest("invalid timeout format"));
    }

    if job_info.target.is_empty() {
        return Err(actix_web::error::ErrorBadRequest(
            "you haven't specified target.zip",
        ));
    }

    if job_info.corpus.is_empty() {
        return Err(actix_web::error::ErrorBadRequest(
            "you haven't specified corpus.zip",
        ));
    }

    Ok(())
}

#[post("/job")]
async fn create_job(
    mut payload: Multipart,
    db_pool: web::Data<SqlitePool>,
    tx: web::Data<Sender<Event>>,
) -> Result<HttpResponse, Error> {
    let mut job_info;
    let guid = Uuid::new_v4().to_string();
    let job_tmp_dir = Path::new(&CONFIG.tmp_dir).join(&guid);
    let job_nfs_dir = Path::new(&CONFIG.nfs_dir).join("jobs").join(&guid);
    let data_dir = job_tmp_dir.join("data/");

    fs::create_dir_all(&data_dir)?;
    fs::create_dir_all(job_tmp_dir.join("/res"))?;
    fs::create_dir_all(job_tmp_dir.join("/crashes"))?;

    match process_job_create(&mut payload, &data_dir).await {
        Ok(_job_info) => {
            job_info = _job_info;
            job_info.guid = guid;
        }
        Err(err) => {
            fs::remove_dir_all(&job_tmp_dir)?;
            return Err(err);
        }
    }

    match sanitize_job_info(&job_info) {
        Ok(_) => {}
        Err(err) => {
            fs::remove_dir_all(&job_tmp_dir)?;
            return Err(err);
        }
    }

    let scheduled_jobs = match Job::schedule_job(&job_info, db_pool.get_ref()).await {
        Ok(res) => res,
        Err(err) => {
            fs::remove_dir_all(&job_tmp_dir)?;
            return Err(actix_web::error::ErrorBadRequest(err));
        }
    };

    fs::rename(job_tmp_dir, job_nfs_dir)?;

    let tx = tx.into_inner();
    for job in scheduled_jobs {
        notify_processor(
            &tx,
            Event::AgentRequest {
                guid: job.agent_guid,
                request: Box::new(Request::JobCreate { job: job.request }),
            },
        )
        .await;
    }

    info!("Created job: {:?}", job_info);

    Ok(HttpResponse::Ok().body(job_info.guid))
}

#[get("/job")]
async fn get_job_stats(db_pool: web::Data<SqlitePool>) -> impl Responder {
    match Job::get_job_stats(db_pool.get_ref()).await {
        Ok(stats) => HttpResponse::Ok().json(stats),
        Err(err) => {
            error!("Error fetching job stats: {}", err);
            HttpResponse::InternalServerError().body("Error fetching job stats")
        }
    }
}

#[get("/job/{guid}")]
async fn get_job(guid: web::Path<String>, db_pool: web::Data<SqlitePool>) -> impl Responder {
    match Job::get_job(&guid, db_pool.get_ref()).await {
        Ok(stats) => HttpResponse::Ok().json(stats),
        Err(err) => {
            error!("Error fetching job: {}", err);
            HttpResponse::InternalServerError().body("Error fetching job")
        }
    }
}

#[get("/jobs")]
async fn get_jobs(db_pool: web::Data<SqlitePool>) -> impl Responder {
    match Job::get_all_collections(db_pool.get_ref()).await {
        Ok(jobs) => HttpResponse::Ok().json(jobs),
        Err(err) => {
            error!("Error fetching jobs: {}", err);
            HttpResponse::InternalServerError().body("Error fetching jobs")
        }
    }
}

#[get("/job/{guid}/stop")]
async fn stop_job(
    guid: web::Path<String>,
    db_pool: web::Data<SqlitePool>,
    tx: web::Data<Sender<Event>>,
) -> impl Responder {
    let tx = tx.into_inner();
    let guid = guid.into_inner();

    match Job::get_job(&guid, db_pool.get_ref()).await {
        Ok(job) => {
            for agent_guid in job.jobs.into_iter().map(|x| x.agent_guid) {
                notify_processor(
                    &tx,
                    Event::AgentRequest {
                        guid: agent_guid,
                        request: Box::new(Request::JobStop { guid: guid.clone() }),
                    },
                )
                .await;
            }

            HttpResponse::Ok().body("Job stop request sent")
        }
        Err(err) => {
            error!("Error fetching job: {}", err);
            HttpResponse::InternalServerError().body("Error fetching job")
        }
    }
}
