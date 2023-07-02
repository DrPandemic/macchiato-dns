use crate::web::AppState;

use actix_web::{dev::ServiceRequest, error, web, Error};
use actix_web_httpauth::extractors::bearer::BearerAuth;

pub async fn validator(req: ServiceRequest, credentials: BearerAuth)
-> Result<ServiceRequest, (Error, ServiceRequest)> {
    let web_password = req.app_data::<web::Data<AppState>>().and_then(|state| {
        state
            .config
            .lock()
            .map(|config| Some(config.web_password.clone()))
            .unwrap_or(None)
    }).flatten();

    if let Some(hash) = web_password {
        if credentials.token() == hash {
            Ok(req)
        } else {
            Err((error::ErrorUnauthorized(""), req))
        }
    } else {
        Err((error::ErrorUnauthorized(""), req))
    }
}
