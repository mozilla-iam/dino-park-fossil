use crate::send::sender::change_display_level;
use crate::send::sender::check_resize_store;
use crate::send::sender::PictureUrl;
use crate::settings::AvatarSettings;
use crate::storage::loader::Loader;
use crate::storage::saver::Saver;
use actix_web::dev::HttpServiceFactory;
use actix_web::error;
use actix_web::http;
use actix_web::middleware::cors::Cors;
use actix_web::web;
use actix_web::web::Data;
use actix_web::web::Json;
use actix_web::web::Path;
use actix_web::Result;
use cis_client::sync::client::CisClientTrait;
use std::sync::Arc;

#[derive(Deserialize)]
struct Uuid {
    uuid: String,
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

fn send_avatar<S: Saver + Clone, L: Loader + Clone>(
    avatar_settings: Data<AvatarSettings>,
    saver: Data<Arc<S>>,
    path: Path<Uuid>,
    body: Json<Avatar>,
) -> Result<Json<PictureUrl>> {
    match check_resize_store(&avatar_settings, &saver, &path.uuid, &body) {
        Ok(v) => Ok(Json(v)),
        Err(e) => Err(error::ErrorBadRequest(e)),
    }
}

fn update_display<T: CisClientTrait + Clone, S: Saver + Clone, L: Loader + Clone>(
    _cis_client: Data<Arc<T>>,
    avatar_settings: Data<AvatarSettings>,
    loader: Data<Arc<L>>,
    saver: Data<Arc<S>>,
    path: Path<Uuid>,
    body: Json<ChangeDisplay>,
) -> Result<Json<PictureUrl>> {
    match change_display_level(&avatar_settings, &loader, &saver, &path.uuid, &body) {
        Ok(v) => Ok(Json(v)),
        Err(e) => Err(error::ErrorBadRequest(e)),
    }
}

pub fn send_app<
    T: CisClientTrait + Clone + Send + Sync + 'static,
    S: Saver + Clone + Send + Sync + 'static,
    L: Loader + Clone + Send + Sync + 'static,
>(
    cis_client: Arc<T>,
    avatar_settings: AvatarSettings,
    saver: Arc<S>,
    loader: Arc<L>,
) -> impl HttpServiceFactory {
    web::scope("/send/")
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
        .data(cis_client)
        .service(
            web::resource("/{uuid]")
                .data(web::JsonConfig::default().limit(1_048_576))
                .route(web::post().to(send_avatar::<S, L>)),
        )
        .service(web::resource("/display/{uuid}").route(web::post().to(update_display::<T, S, L>)))
}
