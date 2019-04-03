extern crate actix;
extern crate actix_web;
extern crate base64;
extern crate chrono;
extern crate cis_client;
extern crate cis_profile;
extern crate config;
extern crate data_url;
extern crate env_logger;
extern crate futures;
extern crate image;
extern crate reqwest;
extern crate rusoto_core;
extern crate rusoto_s3;
extern crate serde;
extern crate sha2;

#[macro_use]
extern crate failure;
#[macro_use]
extern crate log;
#[macro_use]
extern crate serde_derive;

mod retrieve;
mod scale;
mod scope;
mod send;
mod settings;
mod storage;

use crate::storage::loader::S3Loader;
use crate::storage::saver::S3Saver;
use actix_web::middleware;
use actix_web::server;
use cis_client::client::CisClient;
use retrieve::app::retrieve_app;
use scale::app::scale_app;
use send::app::send_app;

fn main() -> Result<(), String> {
    ::std::env::set_var("RUST_LOG", "actix_web=info,dino_park_fossil=info");
    env_logger::init();
    info!("building the fossil");
    let sys = actix::System::new("dino-park-fossil");
    let s = settings::Settings::new().map_err(|e| format!("unable to load settings: {}", e))?;
    let cis_client = CisClient::from_settings(&s.cis)
        .map_err(|e| format!("unable to create cis_client: {}", e))?;
    let avatar_settings = s.avatar.clone();
    let s3_client = rusoto_s3::S3Client::new(rusoto_core::Region::default());
    let saver = S3Saver {
        s3_client: s3_client.clone(),
    };
    let loader = S3Loader {
        s3_client: s3_client.clone(),
    };
    // Start http server
    server::new(move || {
        vec![
            retrieve_app(cis_client.clone(), avatar_settings.clone(), loader.clone())
                .middleware(middleware::Logger::default())
                .boxed(),
            send_app(
                cis_client.clone(),
                avatar_settings.clone(),
                saver.clone(),
                loader.clone(),
            )
            .middleware(middleware::Logger::default())
            .boxed(),
            scale_app()
                .middleware(middleware::Logger::default())
                .boxed(),
        ]
    })
    .bind("0.0.0.0:8083")
    .unwrap()
    .start();

    info!("Started http server");
    let _ = sys.run();
    Ok(())
}
