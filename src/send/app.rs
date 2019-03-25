use crate::send::saver::Saver;
use crate::send::sender::check_resize_store;
use crate::settings::AvatarSettings;
use actix_web::error;
use actix_web::http;
use actix_web::middleware::cors::Cors;
use actix_web::App;
use actix_web::Json;
use actix_web::Path;
use actix_web::Result;
use actix_web::State;
use cis_client::client::CisClientTrait;
use serde_json::Value;

pub struct Sender<T: CisClientTrait + Clone + 'static, S: Saver + Clone + 'static> {
    pub cis_client: T,
    pub avatar_settings: AvatarSettings,
    pub saver: S,
}

#[derive(Deserialize)]
struct Uuid {
    uuid: String,
}

#[derive(Deserialize)]
pub struct Avatar {
    pub data_uri: String,
}

fn send_avatar<T: CisClientTrait + Clone, S: Saver + Clone>(
    state: State<Sender<T, S>>,
    path: Path<Uuid>,
    body: Json<Avatar>,
) -> Result<Json<Value>> {
    match check_resize_store(&state.avatar_settings, &state.saver, &path.uuid, &body) {
        Ok(v) => Ok(Json(v)),
        Err(e) => Err(error::ErrorBadRequest(e)),
    }
}

pub fn send_app<
    T: CisClientTrait + Clone + Send + Sync + 'static,
    S: Saver + Clone + Send + Sync + 'static,
>(
    cis_client: T,
    avatar_settings: AvatarSettings,
    saver: S,
) -> App<Sender<T, S>> {
    App::with_state(Sender {
        cis_client,
        avatar_settings,
        saver,
    })
    .prefix("/avatar/send")
    .configure(|app| {
        Cors::for_app(app)
            .allowed_methods(vec!["POST"])
            .allowed_headers(vec![http::header::AUTHORIZATION, http::header::ACCEPT])
            .allowed_header(http::header::CONTENT_TYPE)
            .max_age(3600)
            .resource("/{uuid}", |r| {
                r.method(http::Method::POST).with(send_avatar)
            })
            .register()
    })
}
