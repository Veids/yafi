use std::env;

use crate::agent_com::Agent;
use actix_files::Files;
use actix_web::{get, web, App, HttpResponse, HttpServer};
use dotenv::dotenv;
use lazy_static::lazy_static;
use sqlx::SqlitePool;
use tera::Tera;
use tokio::sync::mpsc::{self, Sender};

mod agent_com;
mod broker;
mod protos;

use broker::{broker, Event};

lazy_static! {
    pub static ref TEMPLATES: Tera = {
        let mut tera = match Tera::new("templates/**/*") {
            Ok(t) => t,
            Err(e) => {
                println!("Parsing error(s): {}", e);
                ::std::process::exit(1);
            }
        };
        tera.autoescape_on(vec![".html", ".sql"]);
        tera
    };
}

#[get("/")]
async fn index() -> HttpResponse {
    match TEMPLATES.render("index.html", &tera::Context::new()) {
        Ok(t) => HttpResponse::Ok().content_type("text/html").body(t),
        Err(e) => HttpResponse::InternalServerError().body(e.to_string()),
    }
}

#[get("/agents")]
async fn agents() -> HttpResponse {
    match TEMPLATES.render("agents.html", &tera::Context::new()) {
        Ok(t) => HttpResponse::Ok().content_type("text/html").body(t),
        Err(e) => HttpResponse::InternalServerError().body(e.to_string()),
    }
}

#[get("/jobs")]
async fn jobs() -> HttpResponse {
    match TEMPLATES.render("jobs.html", &tera::Context::new()) {
        Ok(t) => HttpResponse::Ok().content_type("text/html").body(t),
        Err(e) => HttpResponse::InternalServerError().body(e.to_string()),
    }
}

#[get("/job/{guid}")]
async fn job() -> HttpResponse {
    match TEMPLATES.render("job_page.html", &tera::Context::new()) {
        Ok(t) => HttpResponse::Ok().content_type("text/html").body(t),
        Err(e) => HttpResponse::InternalServerError().body(e.to_string()),
    }
}

async fn add_existing_agents(tx: &Sender<Event>, db_pool: &SqlitePool) {
    match Agent::get_all(&db_pool).await {
        Ok(agents_vec) => {
            for agent in agents_vec {
                tx.send(Event::NewAgent { guid: agent.guid }).await.unwrap();
            }
        }
        Err(err) => {
            println!(
                "[AgentProcessor.add_existing] error fetching agents: {}",
                err
            );
        }
    }
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();

    let database_url = env::var("DATABASE_URL").expect("Set DATABASE_URL in .env file");
    let db_pool = SqlitePool::connect(&database_url).await.unwrap();

    let (tx, rx) = mpsc::channel::<Event>(100);
    let db = db_pool.clone();
    tokio::spawn(async move { broker(db, rx).await });
    add_existing_agents(&tx, &db_pool).await;

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(db_pool.clone()))
            .app_data(web::Data::new(tx.clone()))
            .service(index)
            .service(agents)
            .service(jobs)
            .service(job)
            .service(
                Files::new("/static", "static/")
                    .prefer_utf8(true)
                    .index_file("static/html/404.html"),
            )
            .configure(agent_com::init)
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
