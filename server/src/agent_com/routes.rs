use std::io::prelude::*;
use std::sync::Arc;
use std::{env, fs, path::Path};

use actix_multipart::{Field, Multipart};
use actix_web::{delete, get, post, web, Error, HttpResponse, Responder};
use futures::StreamExt;
use sqlx::SqlitePool;
use tokio::sync::mpsc::Sender;
use uuid::Uuid;

use crate::agent_com::{Agent, AgentCreateRequest};
use crate::broker::{Event, Request};

#[derive(Debug, Default)]
pub struct JobInfo {
    pub guid: String,
    pub name: String,
    pub description: String,
    pub agent_type: String,
    pub image: String,
    pub cpus: u64,
    pub ram: u64,
    pub timeout: String,
    pub target: String,
    pub corpus: String,
}

pub fn init(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api")
            .service(get_all)
            .service(get_by_guid)
            .service(create)
            .service(delete)
            .service(create_job)
            .service(get_job_stats)
            .service(get_jobs)
            .service(get_job),
    );
}

#[get("/agents")]
async fn get_all(db_pool: web::Data<SqlitePool>) -> impl Responder {
    match Agent::get_all(db_pool.get_ref()).await {
        Ok(agents) => HttpResponse::Ok().json(agents),
        Err(err) => {
            println!("Error fetching agents: {}", err);
            HttpResponse::InternalServerError()
                .body("Error trying to read all agents from database")
        }
    }
}

#[get("/agent/{guid}")]
async fn get_by_guid(guid: web::Path<String>, db_pool: web::Data<SqlitePool>) -> impl Responder {
    match Agent::get_by_guid(&guid.into_inner(), db_pool.get_ref()).await {
        Ok(Some(agent)) => HttpResponse::Ok().json(agent),
        Ok(None) => HttpResponse::NotFound().body("Agent not found"),
        Err(err) => {
            println!("error fetching agent: {}", err);
            HttpResponse::InternalServerError().body("Error trying to read agent from database")
        }
    }
}

async fn notify_processor<T: std::fmt::Debug>(tx: &Arc<Sender<T>>, agent_update: T) {
    match tx.send(agent_update).await {
        Ok(_) => (),
        Err(err) => {
            println!("Error notifying processor: {:?}", err);
        }
    }
}

#[post("/agent")]
async fn create(
    agent_req: web::Json<AgentCreateRequest>,
    db_pool: web::Data<SqlitePool>,
    tx: web::Data<Sender<Event>>,
) -> impl Responder {
    let agent_req = agent_req.into_inner();
    let agent;
    match agent_req.agent_type.as_ref() {
        "linux" => {
            agent = Agent {
                guid: Uuid::new_v4().to_string(),
                description: agent_req.description,
                agent_type: agent_req.agent_type,
                endpoint: agent_req.endpoint,
                status: "init".to_string(),
                ..Default::default()
            };
        }
        _ => {
            return HttpResponse::BadRequest().body("Unsupported agent type");
        }
    }

    match Agent::create(agent, db_pool.get_ref()).await {
        Ok(agent) => {
            notify_processor(
                &tx.into_inner(),
                Event::NewAgent {
                    guid: agent.guid.clone(),
                },
            )
            .await;
            HttpResponse::Ok().json(agent)
        }
        Err(err) => {
            println!("error creating agent: {}", err);
            HttpResponse::InternalServerError().body("Error trying to create new agent")
        }
    }
}

#[delete("/agent/{id}")]
async fn delete(
    guid: web::Path<String>,
    db_pool: web::Data<SqlitePool>,
    tx: web::Data<Sender<Event>>,
) -> impl Responder {
    match Agent::delete(guid.into_inner(), db_pool.get_ref()).await {
        Ok(guid) => {
            notify_processor(&tx.into_inner(), Event::DelAgent { guid: guid.clone() }).await;
            HttpResponse::Ok().body(format!("Succesfully deleted {} agent", guid))
        }
        Err(err) => {
            println!("error deleting agent: {}", err);
            HttpResponse::InternalServerError().body("Todo not found")
        }
    }
}

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
    if job_info.agent_type == "linux" && job_info.image == "" {
        return Err(actix_web::error::ErrorBadRequest(
            "you haven't specified image",
        ));
    }

    if job_info.cpus == 0 {
        return Err(actix_web::error::ErrorBadRequest(
            "you haven't number of cpu cores",
        ));
    }

    if job_info.timeout != "" && job_info.timeout.parse::<humantime::Duration>().is_err() {
        return Err(actix_web::error::ErrorBadRequest("invalid timeout format"));
    }

    if job_info.target == "" {
        return Err(actix_web::error::ErrorBadRequest(
            "you haven't specified target.zip",
        ));
    }

    if job_info.corpus == "" {
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
    let tmp_dir = env::var("TMP_DIR").expect("Set DATABASE_URL in .env file");
    let nfs_dir = env::var("NFS_DIR").expect("Set NFS_DIR in .env file");
    let guid = Uuid::new_v4().to_string();
    let job_tmp_dir = Path::new(&tmp_dir).join(&guid);
    let job_nfs_dir = Path::new(&nfs_dir).join("jobs").join(&guid);
    let data_dir = job_tmp_dir.join("data/");
    fs::create_dir_all(&data_dir)?;

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

    let scheduled_jobs;
    match Agent::schedule_job(&job_info, db_pool.get_ref()).await {
        Ok(res) => scheduled_jobs = res,
        Err(err) => {
            fs::remove_dir_all(&job_tmp_dir)?;
            return Err(actix_web::error::ErrorBadRequest(err));
        }
    }

    fs::rename(job_tmp_dir, job_nfs_dir)?;

    let tx = tx.into_inner();
    for job in scheduled_jobs {
        notify_processor(
            &tx,
            Event::AgentRequest {
                guid: job.agent_guid,
                request: Request::JobCreate { job: job.request },
            },
        )
        .await;
    }

    println!("Created job: {:?}", job_info);

    Ok(HttpResponse::Ok().body(job_info.guid).into())
}

#[get("/job")]
async fn get_job_stats(db_pool: web::Data<SqlitePool>) -> impl Responder {
    match Agent::get_job_stats(db_pool.get_ref()).await {
        Ok(stats) => HttpResponse::Ok().json(stats),
        Err(err) => {
            println!("Error fetching job stats: {}", err);
            HttpResponse::InternalServerError().body("Error fetching job stats")
        }
    }
}

#[get("/job/{guid}")]
async fn get_job(guid: web::Path<String>, db_pool: web::Data<SqlitePool>) -> impl Responder {
    match Agent::get_job(&guid, db_pool.get_ref()).await {
        Ok(stats) => HttpResponse::Ok().json(stats),
        Err(err) => {
            println!("Error fetching job: {}", err);
            HttpResponse::InternalServerError().body("Error fetching job")
        }
    }
}

#[get("/jobs")]
async fn get_jobs(db_pool: web::Data<SqlitePool>) -> impl Responder {
    match Agent::get_all_collections(db_pool.get_ref()).await {
        Ok(jobs) => HttpResponse::Ok().json(jobs),
        Err(err) => {
            println!("Error fetching jobs: {}", err);
            HttpResponse::InternalServerError().body("Error fetching jobs")
        }
    }
}
