use crate::send::sender::change_display_level;
use crate::send::sender::check_resize_store;
use crate::send::sender::PictureUrl;
use crate::settings::AvatarSettings;
use crate::storage::loader::Loader;
use crate::storage::saver::Saver;
use actix_cors::Cors;
use actix_web::dev::HttpServiceFactory;
use actix_web::error;
use actix_web::http;
use actix_web::web;
use actix_web::web::Data;
use actix_web::web::Json;
use actix_web::web::Path;
use actix_web::Error;
use futures::Future;
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
) -> impl Future<Item = Json<PictureUrl>, Error = Error> {
    check_resize_store(&avatar_settings, &saver, &path.uuid, &body)
        .map(Json)
        .map_err(error::ErrorBadRequest)
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
        .service(web::resource("/{uuid}").route(web::post().to_async(send_avatar::<S, L>)))
        .service(
            web::resource("/display/{uuid}").route(web::post().to_async(update_display::<S, L>)),
        )
}
