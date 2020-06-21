use crate::cache::Cache;
use crate::cli::*;
use crate::filter::Filter;

use actix_web::{get, web, App, Error, HttpResponse, HttpServer};
use std::sync::{Arc, Mutex};

const DEFAULT_INTERNAL_ADDRESS_DEBUG: &str = "127.0.0.1:8080";
const DEFAULT_INTERNAL_ADDRESS: &str = "127.0.0.1:80";
const DEFAULT_EXTERNAL_ADDRESS: &str = "0.0.0.0:80";

struct AppState {
    filter: Arc<Mutex<Filter>>,
    cache: Arc<Mutex<Cache>>,
}

#[get("/cache")]
async fn get_cache(data: web::Data<AppState>) -> Result<HttpResponse, Error> {
    let cache = data.cache.lock().unwrap();
    let body = serde_json::to_string(&(*cache)).unwrap();

    // Create response and set content type
    Ok(HttpResponse::Ok().content_type("application/json").body(body))
}

pub async fn start_web(opt: &Opt, filter: Arc<Mutex<Filter>>, cache: Arc<Mutex<Cache>>) -> std::io::Result<()> {
    let state = web::Data::new(AppState {
        filter: filter,
        cache: cache,
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
    HttpServer::new(move || App::new().app_data(state.clone()).service(get_cache))
        .bind(address)?
        .run()
        .await?;
    sys.await?;
    Ok(())
}
