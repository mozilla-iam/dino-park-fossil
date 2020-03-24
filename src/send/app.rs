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
use actix_web::dev::HttpServiceFactory;
use actix_web::error;
use actix_web::http;
use actix_web::web;
use actix_web::web::Bytes;
use actix_web::web::Data;
use actix_web::web::Json;
use actix_web::web::Path;
use actix_web::Error;
use futures::StreamExt;
use futures::TryStreamExt;
use serde::Deserialize;
use serde::Serialize;

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

async fn send_avatar<S: Saver>(
    avatar_settings: Data<AvatarSettings>,
    saver: Data<S>,
    path: Path<Uuid>,
    body: Json<Avatar>,
) -> Result<Json<PictureUrl>, Error> {
    match check_resize_store_data_uri(
        &avatar_settings,
        saver.into_inner(),
        &path.uuid,
        body.into_inner(),
    )
    .await
    {
        Ok(picture_url) => Ok(Json(picture_url)),
        Err(e) => Err(error::ErrorBadRequest(e)),
    }
}

async fn send_save<S: Saver, L: Loader>(
    avatar_settings: Data<AvatarSettings>,
    loader: Data<L>,
    saver: Data<S>,
    path: Path<Uuid>,
    body: Json<Save>,
) -> Result<Json<PictureUrl>, Error> {
    match check_resize_store_intermediate(
        &avatar_settings,
        saver.into_inner(),
        loader.into_inner(),
        &path.uuid,
        body.into_inner(),
    )
    .await
    {
        Ok(picture_url) => Ok(Json(picture_url)),
        Err(e) => Err(error::ErrorBadRequest(e)),
    }
}

async fn send_intermediate<S: Saver>(
    avatar_settings: Data<AvatarSettings>,
    saver: Data<S>,
    mut multipart: Multipart,
) -> Result<Json<Uuid>, Error> {
    if let Some(item) = multipart.next().await {
        let bucket = avatar_settings.s3_bucket.clone();
        let field = item?;
        let buf = field
            .try_fold(
                Vec::<u8>::new(),
                |mut acc: Vec<u8>, bytes: Bytes| async move {
                    acc.extend(bytes.into_iter());
                    Ok(acc)
                },
            )
            .await
            .map_err(error::ErrorBadRequest)?;
        let uuid = store_intermediate(bucket, saver.into_inner(), buf)
            .await
            .map_err(error::ErrorBadRequest)?;
        Ok(Json(Uuid { uuid }))
    } else {
        Err(error::ErrorBadRequest(""))
    }
}

async fn update_display<S: Saver, L: Loader>(
    avatar_settings: Data<AvatarSettings>,
    loader: Data<L>,
    saver: Data<S>,
    path: Path<Uuid>,
    body: Json<ChangeDisplay>,
) -> Result<Json<PictureUrl>, Error> {
    match change_display_level(
        &avatar_settings,
        &loader.into_inner(),
        &saver.into_inner(),
        &path.uuid,
        &body.into_inner(),
    )
    .await
    {
        Ok(picture_url) => Ok(Json(picture_url)),
        Err(e) => Err(error::ErrorBadRequest(e)),
    }
}

pub fn send_app<S: Saver + Send + Sync + 'static, L: Loader + Send + Sync + 'static>(
) -> impl HttpServiceFactory {
    web::scope("/send")
        .wrap(
            Cors::new()
                .allowed_methods(vec!["POST"])
                .allowed_headers(vec![http::header::AUTHORIZATION, http::header::ACCEPT])
                .allowed_header(http::header::CONTENT_TYPE)
                .max_age(3600)
                .finish(),
        )
        .service(web::resource("/intermediate").route(web::post().to(send_intermediate::<S>)))
        .service(web::resource("/{uuid}").route(web::post().to(send_avatar::<S>)))
        .service(web::resource("/save/{uuid}").route(web::post().to(send_save::<S, L>)))
        .service(web::resource("/display/{uuid}").route(web::post().to(update_display::<S, L>)))
}
