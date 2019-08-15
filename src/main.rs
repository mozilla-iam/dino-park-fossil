extern crate actix_cors;
extern crate actix_multipart;
extern crate actix_web;
extern crate base64;
extern crate chrono;
extern crate cis_client;
extern crate cis_profile;
extern crate config;
extern crate data_url;
extern crate dino_park_gate;
extern crate env_logger;
extern crate futures;
extern crate image;
extern crate reqwest;
extern crate rusoto_core;
extern crate rusoto_s3;
extern crate serde;
extern crate sha2;
extern crate uuid;

#[macro_use]
extern crate failure;
#[macro_use]
extern crate failure_derive;
#[macro_use]
extern crate log;
#[macro_use]
extern crate serde_derive;

mod healthz;
mod retrieve;
mod scale;
mod send;
mod settings;
mod storage;

use crate::storage::loader::S3Loader;
use crate::storage::saver::S3Saver;

use actix_web::middleware::Logger;
use actix_web::web;
use actix_web::App;
use actix_web::HttpServer;
use cis_client::CisClient;
use dino_park_gate::provider::Provider;
use failure::Error;
use retrieve::app::retrieve_app;
use scale::app::scale_app;
use send::app::send_app;
use std::sync::Arc;

fn main() -> Result<(), Error> {
    ::std::env::set_var("RUST_LOG", "actix_web=info,dino_park_fossil=info");
    env_logger::init();
    info!("building the fossil");
    let s = settings::Settings::new()?;
    let cis_client = Arc::new(CisClient::from_settings(&s.cis)?);
    let avatar_settings = s.avatar.clone();
    let s3_client = rusoto_s3::S3Client::new(rusoto_core::Region::default());
    let saver = Arc::new(S3Saver {
        s3_client: s3_client.clone(),
    });
    let loader = Arc::new(S3Loader {
        s3_client: s3_client.clone(),
    });
    let provider = Provider::from_issuer("https://auth.mozilla.auth0.com/")?;
    // Start http server
    HttpServer::new(move || {
        App::new()
            .wrap(Logger::default().exclude("/healthz"))
            .service(
                web::scope("/avatar")
                    .service(scale_app())
                    .service(retrieve_app(
                        Arc::clone(&cis_client),
                        avatar_settings.clone(),
                        Arc::clone(&loader),
                        provider.clone(),
                    ))
                    .service(send_app(
                        avatar_settings.clone(),
                        Arc::clone(&saver),
                        Arc::clone(&loader),
                    )),
            )
            .service(healthz::healthz_app())
    })
    .bind("0.0.0.0:8083")?
    .run()
    .map_err(Into::into)
}
