use std::env;
use std::sync::Arc;

use actix_web::{get, web, App, HttpServer, HttpResponse};
use actix_files::Files;
use dotenv::dotenv;
use sqlx::{Pool, Sqlite, SqlitePool};
use tokio::sync::mpsc::{self, Receiver};
use tokio::io::ErrorKind;
use lazy_static::lazy_static;
use tera::Tera;

use agent::job_client::JobClient;
use agent::{JobGuid, JobInfo, JobRequestResult, JobsList};

pub mod agent {
    tonic::include_proto!("agent");
}

mod agent_com;
mod agent_processor;

use agent_processor::AgentProcessor;

// #[tokio::main]
// async fn main() -> Result<(), Box<dyn std::error::Error>> {
//     let mut client = JobClient::connect("http://[::1]:50051").await?;

//     let request = tonic::Request::new(JobGuid{ guid: "None".into()});
//     let response = client.list(request).await?;

//     println!("Response: {:?}", response);

//     Ok(())
// }

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
        Err(e) => HttpResponse::InternalServerError().body(e.to_string())
    }
}

#[get("/agents")]
async fn agents() -> HttpResponse {
    match TEMPLATES.render("agents.html", &tera::Context::new()) {
        Ok(t) => HttpResponse::Ok().content_type("text/html").body(t),
        Err(e) => HttpResponse::InternalServerError().body(e.to_string())
    }
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();

    let database_url = env::var("DATABASE_URL").expect("Set DATABASE_URL in .env file");
    let db_pool = SqlitePool::connect(&database_url).await.unwrap();

    let (tx, mut rx) = mpsc::channel(100);
    let mut agent_processor = AgentProcessor::new(rx, db_pool.clone());

    tokio::spawn(async move { agent_processor.main().await });

    HttpServer::new(move || {
        App::new()
            .data(db_pool.clone())
            .data(tx.clone())
            .service(index)
            .service(agents)
            .service(Files::new("/static", "static/").prefer_utf8(true).index_file("static/html/404.html"))
            .configure(agent_com::init)
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
