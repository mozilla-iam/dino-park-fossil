use crate::send::sender::change_display_level;
use crate::send::sender::check_resize_store;
use crate::send::sender::PictureUrl;
use crate::settings::AvatarSettings;
use crate::storage::loader::Loader;
use crate::storage::saver::Saver;
use actix_web::error;
use actix_web::http;
use actix_web::middleware::cors::Cors;
use actix_web::App;
use actix_web::Json;
use actix_web::Path;
use actix_web::Result;
use actix_web::State;
use cis_client::client::CisClientTrait;

pub struct Sender<
    T: CisClientTrait + Clone + 'static,
    S: Saver + Clone + 'static,
    L: Loader + Clone + 'static,
> {
    pub cis_client: T,
    pub avatar_settings: AvatarSettings,
    pub saver: S,
    pub loader: L,
}

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

fn send_avatar<T: CisClientTrait + Clone, S: Saver + Clone, L: Loader + Clone>(
    state: State<Sender<T, S, L>>,
    path: Path<Uuid>,
    body: Json<Avatar>,
) -> Result<Json<PictureUrl>> {
    match check_resize_store(&state.avatar_settings, &state.saver, &path.uuid, &body) {
        Ok(v) => Ok(Json(v)),
        Err(e) => Err(error::ErrorBadRequest(e)),
    }
}

fn update_display<T: CisClientTrait + Clone, S: Saver + Clone, L: Loader + Clone>(
    state: State<Sender<T, S, L>>,
    path: Path<Uuid>,
    body: Json<ChangeDisplay>,
) -> Result<Json<PictureUrl>> {
    match change_display_level(
        &state.avatar_settings,
        &state.loader,
        &state.saver,
        &path.uuid,
        &body,
    ) {
        Ok(v) => Ok(Json(v)),
        Err(e) => Err(error::ErrorBadRequest(e)),
    }
}

pub fn send_app<
    T: CisClientTrait + Clone + Send + Sync + 'static,
    S: Saver + Clone + Send + Sync + 'static,
    L: Loader + Clone + Send + Sync + 'static,
>(
    cis_client: T,
    avatar_settings: AvatarSettings,
    saver: S,
    loader: L,
) -> App<Sender<T, S, L>> {
    App::with_state(Sender {
        cis_client,
        avatar_settings,
        saver,
        loader,
    })
    .prefix("/avatar/send")
    .configure(|app| {
        Cors::for_app(app)
            .allowed_methods(vec!["POST"])
            .allowed_headers(vec![http::header::AUTHORIZATION, http::header::ACCEPT])
            .allowed_header(http::header::CONTENT_TYPE)
            .max_age(3600)
            .resource("/{uuid}", |r| {
                r.method(http::Method::POST)
                    .with_config(send_avatar, |cfg| {
                        cfg.2.limit(1_048_576);
                    })
            })
            .resource("/display/{uuid}", |r| {
                r.method(http::Method::POST).with(update_display)
            })
            .register()
    })
}
