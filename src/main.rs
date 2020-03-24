#[macro_use]
extern crate failure_derive;

mod healthz;
mod retrieve;
mod send;
mod settings;
mod storage;

use crate::storage::loader::S3Loader;
use crate::storage::saver::S3Saver;
use actix_web::middleware::Logger;
use actix_web::web;
use actix_web::web::Data;
use actix_web::App;
use actix_web::HttpServer;
use cis_client::CisClient;
use dino_park_gate::provider::Provider;
use log::info;
use lru_time_cache::LruCache;
use retrieve::app::retrieve_app;
use send::app::send_app;
use std::io::Error;
use std::io::ErrorKind;
use std::sync::RwLock;

fn map_io_err(e: impl Into<failure::Error>) -> Error {
    Error::new(ErrorKind::Other, e.into())
}

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    ::std::env::set_var("RUST_LOG", "actix_web=info,dino_park_fossil=info");
    env_logger::init();
    info!("building the fossil");
    let s = settings::Settings::new().map_err(map_io_err)?;
    let cis_client = Data::new(CisClient::from_settings(&s.cis).await.map_err(map_io_err)?);
    let avatar_settings = Data::new(s.avatar.clone());
    let s3_client = rusoto_s3::S3Client::new(rusoto_core::Region::default());
    let saver = Data::new(S3Saver {
        s3_client: s3_client.clone(),
    });
    let loader = Data::new(S3Loader {
        s3_client: s3_client.clone(),
    });
    let provider = Provider::from_issuer("https://auth.mozilla.auth0.com/")
        .await
        .map_err(map_io_err)?;

    let time_to_live = ::std::time::Duration::from_secs(60 * 60 * 24);
    let cache = Data::new(RwLock::new(
        LruCache::<String, String>::with_expiry_duration_and_capacity(time_to_live, 2000),
    ));
    // Start http server
    HttpServer::new(move || {
        App::new()
            .wrap(Logger::default().exclude("/healthz"))
            .app_data(loader.clone())
            .app_data(cache.clone())
            .app_data(saver.clone())
            .app_data(cis_client.clone())
            .app_data(avatar_settings.clone())
            .service(
                web::scope("/avatar")
                    .service(retrieve_app::<CisClient, S3Loader<rusoto_s3::S3Client>>(
                        provider.clone(),
                    ))
                    .service(send_app::<
                        S3Saver<rusoto_s3::S3Client>,
                        S3Loader<rusoto_s3::S3Client>,
                    >()),
            )
            .service(healthz::healthz_app())
    })
    .bind("0.0.0.0:8083")?
    .run()
    .await
}
