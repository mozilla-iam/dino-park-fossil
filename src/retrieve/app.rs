use crate::retrieve::retriever::check_and_retrieve_avatar_by_username_from_store;
use crate::retrieve::retriever::retrieve_avatar_from_store;
use crate::scope::Scope;
use crate::settings::AvatarSettings;
use crate::storage::loader::Loader;
use actix_web::error;
use actix_web::http;
use actix_web::http::ContentEncoding;
use actix_web::middleware::cors::Cors;
use actix_web::App;
use actix_web::HttpResponse;
use actix_web::Path;
use actix_web::Query;
use actix_web::Result;
use actix_web::State;
use cis_client::client::CisClientTrait;

pub struct Retriever<T: CisClientTrait + Clone + 'static, L: Loader + Clone + 'static> {
    pub cis_client: T,
    pub avatar_settings: AvatarSettings,
    pub loader: L,
}

#[derive(Deserialize)]
struct Username {
    username: String,
}

#[derive(Deserialize)]
struct Picture {
    picture: String,
}

#[derive(Deserialize)]
struct PictureQuery {
    size: String,
}

fn retrieve_avatar_by_username<T: CisClientTrait + Clone, L: Loader + Clone>(
    state: State<Retriever<T, L>>,
    path: Path<Username>,
    scope: Option<Scope>,
) -> Result<HttpResponse> {
    match check_and_retrieve_avatar_by_username_from_store(
        &state.cis_client,
        &state.avatar_settings,
        &state.loader,
        &path.username,
        &scope,
    ) {
        Ok(b) => Ok(HttpResponse::Ok()
            .content_encoding(ContentEncoding::Identity)
            .header("content-type", "image/png")
            .body(b)),
        Err(e) => Err(error::ErrorBadRequest(e)),
    }
}

fn retrieve_avatar<T: CisClientTrait + Clone, L: Loader + Clone>(
    state: State<Retriever<T, L>>,
    path: Path<Picture>,
    query: Option<Query<PictureQuery>>,
) -> Result<HttpResponse> {
    match retrieve_avatar_from_store(
        &state.avatar_settings,
        &state.loader,
        &path.picture,
        query.as_ref().map(|q| q.size.as_str()),
    ) {
        Ok(b) => Ok(HttpResponse::Ok()
            .content_encoding(ContentEncoding::Identity)
            .header("content-type", "image/png")
            .body(b)),
        Err(e) => Err(error::ErrorBadRequest(e)),
    }
}

pub fn retrieve_app<
    T: CisClientTrait + Clone + Send + Sync + 'static,
    L: Loader + Clone + Send + Sync + 'static,
>(
    cis_client: T,
    avatar_settings: AvatarSettings,
    loader: L,
) -> App<Retriever<T, L>> {
    App::with_state(Retriever {
        cis_client,
        avatar_settings,
        loader,
    })
    .prefix("/avatar/get")
    .configure(|app| {
        Cors::for_app(app)
            .allowed_methods(vec!["GET"])
            .allowed_headers(vec![http::header::AUTHORIZATION, http::header::ACCEPT])
            .allowed_header(http::header::CONTENT_TYPE)
            .max_age(3600)
            .resource("/{username}", |r| {
                r.method(http::Method::GET)
                    .with(retrieve_avatar_by_username)
            })
            .resource("/id/{picture}", |r| {
                r.method(http::Method::GET).with(retrieve_avatar)
            })
            .register()
    })
}
