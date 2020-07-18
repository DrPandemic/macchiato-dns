use crate::cache::Cache;
use crate::cli::*;
use crate::filter::Filter;
use crate::instrumentation::*;
use crate::web_auth::{get_web_password_hash, validator};

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
    pub config: Arc<Mutex<String>>,
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
    opt: &Opt,
    filter: Arc<Mutex<Filter>>,
    cache: Arc<Mutex<Cache>>,
    instrumentation_log: Arc<Mutex<InstrumentationLog>>,
) -> std::io::Result<()> {
    let web_password = get_web_password_hash(&opt);
    let state = web::Data::new(AppState {
        filter: filter,
        cache: cache,
        instrumentation_log: instrumentation_log,
        config: Arc::new(Mutex::new(web_password)),
    });

    let address = if opt.debug {
        DEFAULT_INTERNAL_ADDRESS_DEBUG
    } else if opt.external {
        DEFAULT_EXTERNAL_ADDRESS
    } else {
        DEFAULT_INTERNAL_ADDRESS
    };

    let local = tokio::task::LocalSet::new();
    let sys = actix_rt::System::run_in_tokio("server", &local);

    let mut builder = SslAcceptor::mozilla_intermediate(SslMethod::tls()).unwrap();
    builder.set_private_key_file("key.pem", SslFiletype::PEM).unwrap();
    builder.set_certificate_chain_file("certs.pem").unwrap();

    HttpServer::new(move || {
        let auth = HttpAuthentication::bearer(validator);
        App::new()
            .wrap(auth)
            .app_data(state.clone())
            .service(get_cache)
            .service(get_filter_statistics)
            .service(get_instrumentation)
    })
    .bind_openssl(address, builder)?
    .run()
    .await?;
    sys.await?;
    Ok(())
}
