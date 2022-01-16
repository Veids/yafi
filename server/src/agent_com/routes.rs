use actix_web::{delete, get, post, put, web, HttpResponse, Responder};
use sqlx::SqlitePool;
use tokio::sync::mpsc::Sender;
use uuid::Uuid;

use crate::agent_com::{Agent, AgentRequest, AgentCreateRequest};
use crate::agent_processor::AgentUpdate;

pub fn init(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api")
            .service(get_all)
            .service(get_by_guid)
            .service(create)
            .service(delete)
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

#[post("/agent")]
async fn create(
    agent_req: web::Json<AgentCreateRequest>,
    db_pool: web::Data<SqlitePool>,
    tx: web::Data<Sender<AgentUpdate>>,
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
            };
        },
        _ => {
            return HttpResponse::BadRequest().body("Unsupported agent type");
        }
    }

    match Agent::create(agent, db_pool.get_ref()).await {
        Ok(agent) => {
            tx.send(AgentUpdate {
                guid: agent.guid.clone(),
                update_type: "add".to_string(),
            })
            .await;
            HttpResponse::Ok().json(agent)
        },
        Err(err) => {
            println!("error creating agent: {}", err);
            HttpResponse::InternalServerError().body("Error trying to create new agent")
        }
    }
}

#[delete("/agent/{id}")]
async fn delete(guid: web::Path<String>, db_pool: web::Data<SqlitePool>, tx: web::Data<Sender<AgentUpdate>>) -> impl Responder {
    match Agent::delete(guid.into_inner(), db_pool.get_ref()).await {
        Ok(guid) => {
            tx.send(AgentUpdate{
                guid: guid.clone(),
                update_type: "del".to_string()
            }).await;
            HttpResponse::Ok().body(format!("Succesfully deleted {} agent", guid))
        },
        Err(err) => {
            println!("error deleting agent: {}", err);
            HttpResponse::InternalServerError().body("Todo not found")
        }
    }
}
