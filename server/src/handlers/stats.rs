use crate::config::CONFIG;

use actix_web::{http::header, post, web, HttpResponse, Responder};
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

lazy_static! {
    pub static ref ALLOWED_QUERIES: Vec<String> = vec! {
        "sum by (guid) (fuzzing{type=\"saved_hangs\"})".to_string(),
        "sum by (guid) (fuzzing{type=\"execs_per_sec\"})".to_string(),
        "sum by (guid) (fuzzing{type=\"saved_crashes\"})".to_string(),
        "fuzzing{type=\"edges_found\",banner=\"0\"}".to_string()
    };
    pub static ref QUERY_TIMEOUT: i64 = 10 * 1000;
}

#[derive(Deserialize, Serialize)]
struct StatRequest {
    pub query: String,
}

async fn do_query(prometheus_url: &str, req: &str) -> Result<String, Box<dyn std::error::Error>> {
    let end = chrono::Utc::now();
    let start = end - chrono::Duration::hours(12);

    let params = [
        ("query", req.to_string()),
        ("start", start.to_rfc3339()),
        ("end", end.to_rfc3339()),
        ("step", 30.to_string()),
    ];

    let url = reqwest::Url::parse_with_params(prometheus_url, params)?;
    let res = reqwest::get(url).await?;
    Ok(res.text().await?)
}

async fn do_query_wrap(prometheus_url: &str, req: &str) -> HttpResponse {
    match do_query(prometheus_url, req).await {
        Ok(res) => HttpResponse::Ok()
            .insert_header(header::ContentType(mime::APPLICATION_JSON))
            .body(res),
        Err(err) => {
            HttpResponse::InternalServerError().body(format!("Failed to perform query: {:#}", err))
        }
    }
}

#[post("/stats")]
async fn query_stats(req: web::Json<StatRequest>) -> impl Responder {
    if let Some(prometheus_url) = &CONFIG.prometheus_url {
        if ALLOWED_QUERIES.contains(&req.query) {
            do_query_wrap(prometheus_url, &req.query).await
        } else {
            HttpResponse::BadRequest().body("Provided query is not allowed")
        }
    } else {
        HttpResponse::InternalServerError().body("Stats are not available")
    }
}

#[post("/stats/{guid}")]
async fn query_job_stats(guid: web::Path<String>, req: web::Json<StatRequest>) -> impl Responder {
    if let Some(prometheus_url) = &CONFIG.prometheus_url {
        if let Ok(uuid) = Uuid::try_parse(&guid) {
            let query = match req.query.as_str() {
                "execs_per_sec" => format!("fuzzing{{type=\"execs_per_sec\",guid=\"{uuid}\"}}"),
                "saved_crashes" => format!("fuzzing{{type=\"saved_crashes\",guid=\"{uuid}\"}}"),
                "edges_found" => format!("fuzzing{{type=\"edges_found\",guid=\"{uuid}\"}}"),
                "cycle_done" => format!("fuzzing{{type=\"cycle_done\",guid=\"{uuid}\"}}"),
                _ => return HttpResponse::BadRequest().body("Such query doesn't exist"),
            };
            do_query_wrap(prometheus_url, &query).await
        } else {
            HttpResponse::BadRequest().body("Bad guid")
        }
    } else {
        HttpResponse::InternalServerError().body("Stats are not available")
    }
}
