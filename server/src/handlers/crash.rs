use std::fs;

use actix_http::header::ExtendedValue;
use actix_web::{
    get,
    http::header::{self, DispositionParam, DispositionType},
    web, Error, HttpResponse, Responder,
};
use log::{error, info};
use prost::Message;
use sqlx::SqlitePool;

use crate::{models::Crash, utils::get_job_dir};

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

#[get("/crash")]
async fn get_crash_stats(db_pool: web::Data<SqlitePool>) -> impl Responder {
    match Crash::get_crash_stats(db_pool.get_ref()).await {
        Ok(stats) => HttpResponse::Ok().json(stats),
        Err(err) => {
            error!("Error fetching crash stats: {}", err);
            HttpResponse::InternalServerError().body("Error fetching crash stats")
        }
    }
}

#[get("/crash/{guid}")]
async fn get_crash_info(guid: web::Path<String>, db_pool: web::Data<SqlitePool>) -> impl Responder {
    match Crash::get_crash_info(&guid, db_pool.get_ref()).await {
        Ok(crash) => HttpResponse::Ok().json(crash),
        Err(err) => {
            error!("Error fetching crash info: {}", err);
            HttpResponse::InternalServerError().body("Error fetching crash info")
        }
    }
}

#[get("/crash/{guid}/get")]
async fn get_crash(
    guid: web::Path<String>,
    db_pool: web::Data<SqlitePool>,
) -> Result<HttpResponse, Error> {
    let crash_info = match Crash::get_crash_info(&guid, db_pool.get_ref()).await {
        Ok(crash) => crash,
        Err(err) => {
            error!("Error fetching crash info: {}", err);
            return Err(actix_web::error::ErrorBadGateway(err));
        }
    };

    let crash_path = get_job_dir(&crash_info.collection_guid)
        .join("crashes")
        .join(&crash_info.name);
    info!("Using path {:?}", crash_path);
    let content = fs::read(crash_path)?;
    Ok(HttpResponse::Ok()
        .append_header(header::ContentDisposition {
            disposition: DispositionType::Attachment,
            parameters: vec![DispositionParam::FilenameExt(ExtendedValue {
                charset: header::Charset::Iso_8859_1,
                language_tag: None,
                value: crash_info.name.encode_to_vec(),
            })],
        })
        .content_type("blob")
        .body(content))
}
