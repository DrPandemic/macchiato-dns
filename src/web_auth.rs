use crate::web::AppState;

use actix_web::{dev::ServiceRequest, error, web, Error};
use actix_web_httpauth::extractors::bearer::BearerAuth;
use bcrypt::{hash, verify};
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};

pub async fn validator(req: ServiceRequest, credentials: BearerAuth)
-> Result<ServiceRequest, (Error, ServiceRequest)> {
    let web_password_hash = req.app_data::<web::Data<AppState>>().map(|state| {
        state
            .config
            .lock()
            .map(|config| Some(config.web_password_hash.clone()))
            .unwrap_or(None)
    });

    if let Some(Some(hash)) = web_password_hash {
        if let Ok(true) = verify(credentials.token(), &hash) {
            Ok(req)
        } else {
            Err((error::ErrorUnauthorized(""), req))
        }
    } else {
        Err((error::ErrorUnauthorized(""), req))
    }
}

pub fn get_web_password_hash(maybe_password: Option<String>) -> String {
    let password = maybe_password.unwrap_or_else(|| thread_rng().sample_iter(&Alphanumeric).take(30).collect());

    println!("The web password is {}", password);

    hash(password, 6).unwrap()
}
