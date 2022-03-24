use actix_web::{get, HttpResponse};
use lazy_static::lazy_static;
use log::error;
use tera::Tera;

lazy_static! {
    pub static ref TEMPLATES: Tera = {
        let mut tera = match Tera::new("templates/**/*") {
            Ok(t) => t,
            Err(e) => {
                error!("Parsing error(s): {}", e);
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

#[get("/crashes")]
async fn crashes() -> HttpResponse {
    match TEMPLATES.render("crashes.html", &tera::Context::new()) {
        Ok(t) => HttpResponse::Ok().content_type("text/html").body(t),
        Err(e) => HttpResponse::InternalServerError().body(e.to_string()),
    }
}

#[get("/crash/{guid}")]
async fn crash() -> HttpResponse {
    match TEMPLATES.render("crash_page.html", &tera::Context::new()) {
        Ok(t) => HttpResponse::Ok().content_type("text/html").body(t),
        Err(e) => HttpResponse::InternalServerError().body(e.to_string()),
    }
}
