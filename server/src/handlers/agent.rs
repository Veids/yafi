use crate::broker::Event;
use crate::models::{Agent, AgentCreateRequest};
use crate::utils::notify_processor;

use actix_web::{delete, get, post, web, HttpResponse, Responder};
use log::error;
use sqlx::SqlitePool;
use tokio::sync::mpsc::Sender;
use uuid::Uuid;

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
    pub crash_auto_analyze: bool,
}

#[get("/agents")]
async fn get_all(db_pool: web::Data<SqlitePool>) -> impl Responder {
    match Agent::get_all(db_pool.get_ref()).await {
        Ok(agents) => HttpResponse::Ok().json(agents),
        Err(err) => {
            error!("Error fetching agents: {}", err);
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
            error!("Failed to fetch agent: {}", err);
            HttpResponse::InternalServerError().body("Error trying to read agent from database")
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
    let agent = match agent_req.agent_type.as_ref() {
        "linux" => Agent {
            guid: Uuid::new_v4().to_string(),
            description: agent_req.description,
            agent_type: agent_req.agent_type,
            endpoint: agent_req.endpoint,
            status: "init".to_string(),
            ..Default::default()
        },
        _ => {
            return HttpResponse::BadRequest().body("Unsupported agent type");
        }
    };

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
            error!("error creating agent: {}", err);
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
            error!("error deleting agent: {}", err);
            HttpResponse::InternalServerError().body("Todo not found")
        }
    }
}
