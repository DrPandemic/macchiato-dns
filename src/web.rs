use crate::cache::Cache;
use crate::config::Config;
use crate::filter::Filter;
use crate::filter_statistics::FilterStatistics;
use crate::instrumentation::*;
use crate::prometheus::metrics;
use crate::web_auth::validator;

use actix_files as fs;
use actix_web::{delete, error, get, middleware, post, web, App, Error, HttpResponse, HttpServer};
use actix_web_httpauth::middleware::HttpAuthentication;
use serde::Deserialize;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

const DEFAULT_INTERNAL_ADDRESS_DEBUG: &str = "127.0.0.1:8080";
const DEFAULT_INTERNAL_ADDRESS: &str = "127.0.0.1:80";
const DEFAULT_EXTERNAL_ADDRESS: &str = "0.0.0.0:80";

pub struct AppState {
    filter: Arc<Mutex<Filter>>,
    cache: Arc<Mutex<Cache>>,
    pub instrumentation_log: Arc<Mutex<InstrumentationLog>>,
    pub config: Arc<Mutex<Config>>,
    pub filter_update_channel: Arc<Mutex<Sender<()>>>,
}

#[get("/cache")]
async fn get_cache(data: web::Data<AppState>) -> Result<HttpResponse, Error> {
    let cache = data.cache.lock().unwrap();
    let body = serde_json::to_string(&(*cache)).unwrap();

    Ok(HttpResponse::Ok().content_type("application/json").body(body))
}

#[derive(serde::Serialize)]
struct StatisticsBody<'a> {
    pub statistics: &'a FilterStatistics,
    pub size: usize,
    pub created_at: SystemTime,
}

#[get("/filter")]
async fn get_filter(data: web::Data<AppState>) -> Result<HttpResponse, Error> {
    let filter = data.filter.lock().unwrap();

    let content = StatisticsBody {
        statistics: &filter.statistics,
        size: filter.size,
        created_at: filter.created_at,
    };
    let body = serde_json::to_string(&content)?;

    Ok(HttpResponse::Ok().content_type("application/json").body(body))
}

#[get("/instrumentation")]
async fn get_instrumentation(data: web::Data<AppState>) -> Result<HttpResponse, Error> {
    let instrumentation_log = data.instrumentation_log.lock().unwrap();
    let body = serde_json::to_string(&(*instrumentation_log)).unwrap();

    Ok(HttpResponse::Ok().content_type("application/json").body(body))
}

#[get("/allowed-domains")]
async fn get_allowed_domains(data: web::Data<AppState>) -> Result<HttpResponse, Error> {
    let config = data.config.lock().unwrap();
    let body = serde_json::to_string(&config.allowed_domains).unwrap();

    Ok(HttpResponse::Ok().content_type("application/json").body(body))
}

#[derive(Deserialize)]
struct Domain {
    name: String,
}

#[post("/allowed-domains")]
async fn post_allowed_domains(domain: web::Json<Domain>, data: web::Data<AppState>) -> actix_web::Result<String> {
    let mut config = data.config.lock().unwrap();

    config.allowed_domains.push(domain.name.clone());
    config.allowed_domains.sort();
    let saved = config.save();

    match saved {
        Err(err) => Err(error::ErrorInternalServerError(err)),
        _ => Ok("{}".to_string()),
    }
}

#[post("/update-filter")]
async fn post_update_filter(data: web::Data<AppState>) -> actix_web::Result<String> {
    let result = data.filter_update_channel.lock().unwrap().send(());
    if result.is_ok() {
        Ok("{}".to_string())
    } else {
        Err(error::ErrorServiceUnavailable(
            "{\"error\": \"Can't update filter\"}".to_string(),
        ))
    }
}

#[delete("/allowed-domains")]
async fn delete_allowed_domains(domain: web::Json<Domain>, data: web::Data<AppState>) -> actix_web::Result<String> {
    let mut config = data.config.lock().unwrap();

    config.allowed_domains.retain(|d| d != &domain.name.clone());

    let saved = config.save();

    match saved {
        Err(err) => Err(error::ErrorInternalServerError(err)),
        _ => Ok("{}".to_string()),
    }
}

#[get("/auto-update-filter")]
async fn get_auto_update_filter(data: web::Data<AppState>) -> Result<HttpResponse, Error> {
    let config = data.config.lock().unwrap();
    let body = serde_json::to_string(&config.auto_update).unwrap();

    Ok(HttpResponse::Ok().content_type("application/json").body(body))
}

#[derive(Deserialize)]
struct AutoUpdate {
    auto_update: Option<u64>,
}

#[post("/auto-update-filter")]
async fn post_auto_update_filter(
    auto_udapte: web::Json<AutoUpdate>,
    data: web::Data<AppState>,
) -> actix_web::Result<String> {
    let mut config = data.config.lock().unwrap();
    config.auto_update = auto_udapte.auto_update;

    let saved = config.save();

    match saved {
        Err(err) => Err(error::ErrorInternalServerError(err)),
        _ => Ok("{}".to_string()),
    }
}

#[get("/overrides")]
async fn get_overrides(data: web::Data<AppState>) -> Result<HttpResponse, Error> {
    let config = data.config.lock().unwrap();
    let body = serde_json::to_string(&config.overrides).unwrap();

    Ok(HttpResponse::Ok().content_type("application/json").body(body))
}

#[delete("/overrides")]
async fn delete_overrides(domain: web::Json<Domain>, data: web::Data<AppState>) -> actix_web::Result<String> {
    let mut config = data.config.lock().unwrap();

    config.overrides.remove(&domain.name);

    let saved = config.save();

    match saved {
        Err(err) => Err(error::ErrorInternalServerError(err)),
        _ => Ok("{}".to_string()),
    }
}

#[derive(Deserialize)]
struct DomainWithAddress {
    name: String,
    address: String,
}
#[post("/overrides")]
async fn post_overrides(domain: web::Json<DomainWithAddress>, data: web::Data<AppState>) -> actix_web::Result<String> {
    let mut config = data.config.lock().unwrap();
    let address: Result<Vec<u8>, std::num::ParseIntError> =
        domain.address.split(".").map(|s| s.parse::<u8>()).collect();

    match address {
        Ok(address) => {
            config.overrides.insert(domain.name.clone(), address);
            let saved = config.save();

            match saved {
                Err(err) => Err(error::ErrorInternalServerError(err)),
                _ => Ok("{}".to_string()),
            }
        }
        Err(err) => Err(error::ErrorBadRequest(err)),
    }
}

pub async fn start_web(
    config: Arc<Mutex<Config>>,
    filter: Arc<Mutex<Filter>>,
    cache: Arc<Mutex<Cache>>,
    instrumentation_log: Arc<Mutex<InstrumentationLog>>,
    filter_update_channel: Arc<Mutex<Sender<()>>>,
) -> std::io::Result<()> {
    let address = {
        let locked_config = config.lock().unwrap();
        if locked_config.debug {
            DEFAULT_INTERNAL_ADDRESS_DEBUG
        } else if locked_config.external {
            DEFAULT_EXTERNAL_ADDRESS
        } else {
            DEFAULT_INTERNAL_ADDRESS
        }
    };

    let state = web::Data::new(AppState {
        filter,
        cache,
        instrumentation_log,
        config,
        filter_update_channel,
    });

    HttpServer::new(move || {
        App::new()
            .app_data(state.clone())
            .wrap(middleware::Compress::default())
            .service(
                web::scope("/api/1")
                    .wrap(HttpAuthentication::bearer(validator))
                    .service(get_cache)
                    .service(get_filter)
                    .service(get_instrumentation)
                    .service(get_allowed_domains)
                    .service(get_auto_update_filter)
                    .service(get_overrides)
                    .service(post_auto_update_filter)
                    .service(post_allowed_domains)
                    .service(post_update_filter)
                    .service(post_overrides)
                    .service(delete_allowed_domains)
                    .service(delete_overrides)
                    .service(metrics),
            )
            .service(fs::Files::new("/", "./static").index_file("index.html"))
    })
    .bind(address)?
    .run()
    .await?;
    Ok(())
}
