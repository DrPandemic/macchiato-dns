use crate::cache::Cache;
use crate::config::Config;
use crate::filter::Filter;
use crate::instrumentation::*;
use crate::web_auth::validator;
use crate::filter_statistics::FilterStatistics;
use crate::prometheus::metrics;

use actix_files as fs;
use actix_web::{delete, get, post, web, error, middleware, App, Error, HttpResponse, HttpServer};
use actix_web_httpauth::middleware::HttpAuthentication;
use rustls::internal::pemfile::{certs, pkcs8_private_keys};
use rustls::{NoClientAuth, ServerConfig};
use serde::Deserialize;
use std::fs::File;
use std::io::BufReader;
use std::sync::{Arc, Mutex};
use std::time::SystemTime;
use std::sync::mpsc::Sender;

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
        _ => Ok("{}".to_string())
    }
}

#[post("/update-filter")]
async fn post_update_filter(data: web::Data<AppState>) -> actix_web::Result<String> {
    let result = data.filter_update_channel.lock().unwrap().send(());
    if result.is_ok() {
        Ok("{}".to_string())
    } else {
        Err(error::ErrorServiceUnavailable("{\"error\": \"Can't update filter\"}".to_string()))
    }
}

#[delete("/allowed-domains")]
async fn delete_allowed_domains(domain: web::Json<Domain>, data: web::Data<AppState>) -> actix_web::Result<String> {
    let mut config = data.config.lock().unwrap();

    config.allowed_domains.retain(|d| d != &domain.name.clone());

    let saved = config.save();

    match saved {
        Err(err) => Err(error::ErrorInternalServerError(err)),
        _ => Ok("{}".to_string())
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
async fn post_auto_update_filter(auto_udapte: web::Json<AutoUpdate>, data: web::Data<AppState>) -> actix_web::Result<String> {
    let mut config = data.config.lock().unwrap();
    config.auto_update = auto_udapte.auto_update;

    let saved = config.save();

    match saved {
        Err(err) => Err(error::ErrorInternalServerError(err)),
        _ => Ok("{}".to_string())
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

    let state = web::Data::new(AppState { filter, cache, instrumentation_log, config, filter_update_channel });

    let local = tokio::task::LocalSet::new();
    let sys = actix_rt::System::run_in_tokio("server", &local);

    let mut server_config = ServerConfig::new(NoClientAuth::new());
    let cert_file = &mut BufReader::new(File::open("ssl/certs.pem").unwrap());
    let key_file = &mut BufReader::new(File::open("ssl/key.pem").unwrap());
    let cert_chain = certs(cert_file).unwrap();
    let mut keys = pkcs8_private_keys(key_file).unwrap();
    server_config.set_single_cert(cert_chain, keys.remove(0)).unwrap();

    HttpServer::new(move || {
        let auth = HttpAuthentication::bearer(validator);
        App::new()
            .app_data(state.clone())
            .wrap(middleware::Compress::default())
            .service(
                web::scope("/api/1")
                    .wrap(auth)
                    .service(get_cache)
                    .service(get_filter)
                    .service(get_instrumentation)
                    .service(get_allowed_domains)
                    .service(get_auto_update_filter)
                    .service(post_auto_update_filter)
                    .service(post_allowed_domains)
                    .service(post_update_filter)
                    .service(delete_allowed_domains)
                    .service(metrics)
            )
            .service(fs::Files::new("/", "./static").index_file("index.html"))
    })
    .bind_rustls(address, server_config)?
    .run()
    .await?;
    sys.await?;
    Ok(())
}
