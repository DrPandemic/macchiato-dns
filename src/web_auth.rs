use crate::cli::Opt;
use crate::web::AppState;

use actix_web::{dev::ServiceRequest, error, Error};
use actix_web_httpauth::extractors::bearer::BearerAuth;
use bcrypt::{hash, verify};
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};

pub async fn validator(req: ServiceRequest, credentials: BearerAuth) -> Result<ServiceRequest, Error> {
    let web_password_hash = req.app_data::<AppState>().map(|state| {
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
            Err(error::ErrorUnauthorized(""))
        }
    } else {
        Err(error::ErrorUnauthorized(""))
    }
}

pub fn get_web_password_hash(opt: &Opt) -> String {
    let password = opt
        .web_password
        .as_ref()
        .map(|password| password.to_string())
        .unwrap_or_else(|| thread_rng().sample_iter(&Alphanumeric).take(30).collect());

    println!("The web password is {}", password);

    hash(password, 6).unwrap()
}
