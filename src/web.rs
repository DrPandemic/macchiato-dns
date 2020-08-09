use crate::cache::Cache;
use crate::config::Config;
use crate::filter::Filter;
use crate::instrumentation::*;
use crate::web_auth::validator;

use actix_files as fs;
use actix_web::{get, web, App, Error, HttpResponse, HttpServer};
use actix_web_httpauth::middleware::HttpAuthentication;
use openssl::ssl::{SslAcceptor, SslFiletype, SslMethod};
use std::sync::{Arc, Mutex};

const DEFAULT_INTERNAL_ADDRESS_DEBUG: &str = "127.0.0.1:8080";
const DEFAULT_INTERNAL_ADDRESS: &str = "127.0.0.1:80";
const DEFAULT_EXTERNAL_ADDRESS: &str = "0.0.0.0:80";

pub struct AppState {
    filter: Arc<Mutex<Filter>>,
    cache: Arc<Mutex<Cache>>,
    instrumentation_log: Arc<Mutex<InstrumentationLog>>,
    pub config: Arc<Mutex<Config>>,
}

#[get("/cache")]
async fn get_cache(data: web::Data<AppState>) -> Result<HttpResponse, Error> {
    let cache = data.cache.lock().unwrap();
    let body = serde_json::to_string(&(*cache)).unwrap();

    Ok(HttpResponse::Ok().content_type("application/json").body(body))
}

#[get("/filter-statistics")]
async fn get_filter_statistics(data: web::Data<AppState>) -> Result<HttpResponse, Error> {
    let filter = data.filter.lock().unwrap();
    let body = serde_json::to_string(&filter.statistics).unwrap();

    Ok(HttpResponse::Ok().content_type("application/json").body(body))
}

#[get("/instrumentation")]
async fn get_instrumentation(data: web::Data<AppState>) -> Result<HttpResponse, Error> {
    let instrumentation_log = data.instrumentation_log.lock().unwrap();
    let body = serde_json::to_string(&(*instrumentation_log)).unwrap();

    Ok(HttpResponse::Ok().content_type("application/json").body(body))
}

pub async fn start_web(
    config: Config,
    filter: Arc<Mutex<Filter>>,
    cache: Arc<Mutex<Cache>>,
    instrumentation_log: Arc<Mutex<InstrumentationLog>>,
) -> std::io::Result<()> {
    let address = if config.debug {
        DEFAULT_INTERNAL_ADDRESS_DEBUG
    } else if config.external {
        DEFAULT_EXTERNAL_ADDRESS
    } else {
        DEFAULT_INTERNAL_ADDRESS
    };

    let state = web::Data::new(AppState {
        filter: filter,
        cache: cache,
        instrumentation_log: instrumentation_log,
        config: Arc::new(Mutex::new(config)),
    });

    let local = tokio::task::LocalSet::new();
    let sys = actix_rt::System::run_in_tokio("server", &local);

    let mut builder = SslAcceptor::mozilla_intermediate(SslMethod::tls()).unwrap();
    builder.set_private_key_file("ssl/key.pem", SslFiletype::PEM).unwrap();
    builder.set_certificate_chain_file("ssl/certs.pem").unwrap();

    HttpServer::new(move || {
        let auth = HttpAuthentication::bearer(validator);
        App::new()
            .app_data(state.clone())
            .service(
                web::scope("/api/1")
                    .wrap(auth)
                    .service(get_cache)
                    .service(get_filter_statistics)
                    .service(get_instrumentation),
            )
            .service(fs::Files::new("/", "./static").index_file("index.html"))
    })
    .bind_openssl(address, builder)?
    .run()
    .await?;
    sys.await?;
    Ok(())
}
