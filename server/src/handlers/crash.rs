use actix_web::{get, web, HttpResponse, Responder};
use log::error;
use sqlx::SqlitePool;

use crate::models::Crash;

#[get("/crashes")]
async fn get_crashes(db_pool: web::Data<SqlitePool>) -> impl Responder {
    match Crash::get_all_crashes(db_pool.get_ref()).await {
        Ok(crashes) => HttpResponse::Ok().json(crashes),
        Err(err) => {
            error!("Error fetching crashes: {}", err);
            HttpResponse::InternalServerError().body("Error fetching crashes")
        }
    }
}
