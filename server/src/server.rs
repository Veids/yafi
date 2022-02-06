use crate::broker::{broker, Event};
use crate::config::CONFIG;
use crate::models::Agent;
use crate::routes::routes;

use actix_web::{middleware::Logger, web, App, HttpServer};
use dotenv::dotenv;
use log::error;
use sqlx::SqlitePool;
use tokio::sync::mpsc::{self, Sender};

async fn add_existing_agents(tx: &Sender<Event>, db_pool: &SqlitePool) {
    match Agent::get_all(db_pool).await {
        Ok(agents_vec) => {
            for agent in agents_vec {
                tx.send(Event::NewAgent { guid: agent.guid }).await.unwrap();
            }
        }
        Err(err) => {
            error!(
                "[AgentProcessor.add_existing] error fetching agents: {}",
                err
            );
        }
    }
}

pub async fn server() -> std::io::Result<()> {
    dotenv().ok();
    env_logger::init();

    let db_pool = SqlitePool::connect(&CONFIG.database_url).await.unwrap();

    let (tx, rx) = mpsc::channel::<Event>(100);
    let db = db_pool.clone();
    tokio::spawn(async move { broker(db, rx).await });
    add_existing_agents(&tx, &db_pool).await;

    HttpServer::new(move || {
        App::new()
            .wrap(Logger::default())
            .app_data(web::Data::new(db_pool.clone()))
            .app_data(web::Data::new(tx.clone()))
            .configure(routes)
    })
    .bind(&CONFIG.sap_server_listen)?
    .run()
    .await
}
