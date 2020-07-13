use crate::cache::Cache;
use crate::cli::*;
use crate::filter::Filter;
use crate::instrumentation::*;

use actix_web::{get, web, App, Error, HttpResponse, HttpServer};
use std::sync::{Arc, Mutex};

const DEFAULT_INTERNAL_ADDRESS_DEBUG: &str = "127.0.0.1:8080";
const DEFAULT_INTERNAL_ADDRESS: &str = "127.0.0.1:80";
const DEFAULT_EXTERNAL_ADDRESS: &str = "0.0.0.0:80";

struct AppState {
    filter: Arc<Mutex<Filter>>,
    cache: Arc<Mutex<Cache>>,
    instrumentation_log: Arc<Mutex<InstrumentationLog>>,
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
    let state = web::Data::new(AppState {
        filter: filter,
        cache: cache,
        instrumentation_log: instrumentation_log,
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
    HttpServer::new(move || {
        App::new()
            .app_data(state.clone())
            .service(get_cache)
            .service(get_filter_statistics)
            .service(get_instrumentation)
    })
    .bind(address)?
    .run()
    .await?;
    sys.await?;
    Ok(())
}
