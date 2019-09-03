use crate::send::sender::change_display_level;
use crate::send::sender::check_resize_store_data_uri;
use crate::send::sender::check_resize_store_intermediate;
use crate::send::sender::store_intermediate;
use crate::send::sender::PictureUrl;
use crate::settings::AvatarSettings;
use crate::storage::loader::Loader;
use crate::storage::saver::Saver;
use actix_cors::Cors;
use actix_multipart::Multipart;
use actix_multipart::MultipartError;
use actix_web::dev::HttpServiceFactory;
use actix_web::error;
use actix_web::http;
use actix_web::web;
use actix_web::web::Bytes;
use actix_web::web::Data;
use actix_web::web::Json;
use actix_web::web::Path;
use actix_web::Error;
use futures::future;
use futures::Future;
use futures::Stream;
use serde::Deserialize;
use serde::Serialize;
use std::sync::Arc;

#[derive(Deserialize, Serialize)]
struct Uuid {
    uuid: String,
}

#[derive(Deserialize)]
pub struct Save {
    pub intermediate: String,
    pub display: String,
    pub old_url: Option<String>,
}

#[derive(Deserialize)]
pub struct Avatar {
    pub data_uri: String,
    pub display: String,
    pub old_url: Option<String>,
}

#[derive(Deserialize)]
pub struct ChangeDisplay {
    pub display: String,
    pub old_url: String,
}

fn send_avatar<S: Saver + Clone>(
    avatar_settings: Data<AvatarSettings>,
    saver: Data<Arc<S>>,
    path: Path<Uuid>,
    body: Json<Avatar>,
) -> impl Future<Item = Json<PictureUrl>, Error = Error> {
    check_resize_store_data_uri(&avatar_settings, &saver, &path.uuid, body.0)
        .map(Json)
        .map_err(error::ErrorBadRequest)
}

fn send_save<S: Saver + Clone, L: Loader + Clone>(
    avatar_settings: Data<AvatarSettings>,
    loader: Data<Arc<L>>,
    saver: Data<Arc<S>>,
    path: Path<Uuid>,
    body: Json<Save>,
) -> impl Future<Item = Json<PictureUrl>, Error = Error> {
    check_resize_store_intermediate(&avatar_settings, &saver, &loader, &path.uuid, body.0)
        .map(Json)
        .map_err(error::ErrorBadRequest)
}

fn send_intermediate<S: Saver + Clone>(
    avatar_settings: Data<AvatarSettings>,
    saver: Data<Arc<S>>,
    multipart: Multipart,
) -> impl Future<Item = Json<Uuid>, Error = Error> {
    multipart
        .map(move |field| {
            let saver = Arc::clone(&saver);
            let bucket = avatar_settings.s3_bucket.clone();
            field
                .fold(Vec::<u8>::new(), |mut acc: Vec<u8>, bytes: Bytes| {
                    acc.extend(bytes.into_iter());
                    future::result(Ok(acc).map_err(|e| {
                        println!("file.write_all failed: {:?}", e);
                        MultipartError::Payload(error::PayloadError::Io(e))
                    }))
                })
                .map_err(|e| {
                    println!("failed multipart for intermediate, {:?}", e);
                    error::ErrorBadRequest(e)
                })
                .and_then(move |buf: Vec<u8>| {
                    store_intermediate(bucket, saver, buf).map_err(error::ErrorBadRequest)
                })
                .into_stream()
        })
        .map_err(error::ErrorBadRequest)
        .flatten()
        .collect()
        .map(|mut v| v.pop().unwrap_or_default())
        .map_err(Into::into)
        .map(|uuid| Json(Uuid { uuid }))
}

fn update_display<S: Saver + Clone, L: Loader + Clone>(
    avatar_settings: Data<AvatarSettings>,
    loader: Data<Arc<L>>,
    saver: Data<Arc<S>>,
    path: Path<Uuid>,
    body: Json<ChangeDisplay>,
) -> impl Future<Item = Json<PictureUrl>, Error = Error> {
    change_display_level(&avatar_settings, &loader, &saver, &path.uuid, &body)
        .map(Json)
        .map_err(error::ErrorBadRequest)
}

pub fn send_app<
    S: Saver + Clone + Send + Sync + 'static,
    L: Loader + Clone + Send + Sync + 'static,
>(
    avatar_settings: AvatarSettings,
    saver: Arc<S>,
    loader: Arc<L>,
) -> impl HttpServiceFactory {
    web::scope("/send")
        .wrap(
            Cors::new()
                .allowed_methods(vec!["POST"])
                .allowed_headers(vec![http::header::AUTHORIZATION, http::header::ACCEPT])
                .allowed_header(http::header::CONTENT_TYPE)
                .max_age(3600),
        )
        .data(loader)
        .data(saver)
        .data(avatar_settings)
        .data(web::JsonConfig::default().limit(1_048_576))
        .service(web::resource("/intermediate").route(web::post().to_async(send_intermediate::<S>)))
        .service(web::resource("/{uuid}").route(web::post().to_async(send_avatar::<S>)))
        .service(web::resource("/save/{uuid}").route(web::post().to_async(send_save::<S, L>)))
        .service(
            web::resource("/display/{uuid}").route(web::post().to_async(update_display::<S, L>)),
        )
}
