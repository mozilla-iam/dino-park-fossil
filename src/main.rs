#[macro_use]
extern crate failure_derive;

mod error;
mod healthz;
mod retrieve;
mod send;
mod settings;
mod storage;

use actix_web::middleware::Logger;
use actix_web::web;
use actix_web::web::Data;
use actix_web::App;
use actix_web::HttpServer;
use cis_client::CisClient;
use dino_park_gate::provider::Provider;
use dino_park_gate::scope::ScopeAndUserAuth;
use log::info;
use lru_time_cache::LruCache;
use retrieve::app::retrieve_app;
use send::app::internal_send_app;
use send::app::send_app;
use std::io::Error;
use std::io::ErrorKind;
use std::sync::Mutex;

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
    let provider = Provider::from_issuer(&s.auth).await.map_err(map_io_err)?;

    let time_to_live = ::std::time::Duration::from_secs(60 * 60 * 24);
    let cache = Data::new(Mutex::new(
        LruCache::<String, String>::with_expiry_duration_and_capacity(time_to_live, 2000),
    ));
    // Start http server
    HttpServer::new(move || {
        let scope_middleware = ScopeAndUserAuth::new(provider.clone()).public();

        #[cfg(feature = "local-fs")]
        {
            use crate::storage::loader::filesystem::FilesystemLoader;
            use crate::storage::saver::filesystem::FilesystemSaver;
            use std::path::PathBuf;
            use std::sync::Arc;

            let mut path = PathBuf::new();
            path.push("./files");

            let saver = Data::new(FilesystemSaver {
                path: Arc::new(path.clone()),
            });
            let loader = Data::new(FilesystemLoader {
                path: Arc::new(path.clone()),
            });

            App::new()
                .wrap(Logger::default().exclude("/healthz"))
                .app_data(loader.clone())
                .app_data(cache.clone())
                .app_data(saver.clone())
                .app_data(cis_client.clone())
                .app_data(avatar_settings.clone())
                .service(
                    web::scope("/avatar")
                        .service(retrieve_app::<CisClient, FilesystemLoader>(
                            scope_middleware.clone(),
                        ))
                        .service(send_app::<FilesystemSaver, FilesystemLoader>(
                            scope_middleware,
                        )),
                )
                .service(internal_send_app::<FilesystemSaver, FilesystemLoader>())
                .service(healthz::healthz_app())
        }

        #[cfg(not(feature = "local-fs"))]
        {
            use crate::storage::loader::s3::S3Loader;
            use crate::storage::saver::s3::S3Saver;
            use rusoto_s3::S3Client;

            let s3_client = S3Client::new(rusoto_core::Region::default());
            let saver = Data::new(S3Saver {
                s3_client: s3_client.clone(),
            });
            let loader = Data::new(S3Loader { s3_client });

            App::new()
                .wrap(Logger::default().exclude("/healthz"))
                .app_data(loader)
                .app_data(cache.clone())
                .app_data(saver)
                .app_data(cis_client.clone())
                .app_data(avatar_settings.clone())
                .service(
                    web::scope("/avatar")
                        .service(retrieve_app::<CisClient, S3Loader>(
                            scope_middleware.clone(),
                        ))
                        .service(send_app::<S3Saver, S3Loader>(scope_middleware)),
                )
                .service(internal_send_app::<S3Saver, S3Loader>())
                .service(healthz::healthz_app())
        }
    })
    .bind("0.0.0.0:8083")?
    .run()
    .await
}
