use crate::retrieve::retriever::retrieve_avatar_from_store;
use crate::retrieve::uuid::get_uuid;
use crate::settings::AvatarSettings;
use crate::storage::loader::Loader;
use actix_cors::Cors;
use actix_web::dev::HttpServiceFactory;
use actix_web::error;
use actix_web::guard;
use actix_web::http;
use actix_web::http::ContentEncoding;
use actix_web::middleware::BodyEncoding;
use actix_web::web;
use actix_web::web::Data;
use actix_web::web::Path;
use actix_web::web::Query;
use actix_web::Error;
use actix_web::HttpResponse;
use cis_client::AsyncCisClientTrait;
use cis_profile::schema::Display;
use dino_park_gate::provider::Provider;
use dino_park_gate::scope::ScopeAndUser;
use dino_park_gate::scope::ScopeAndUserAuth;
use futures::Future;
use log::info;
use log::warn;
use lru_time_cache::LruCache;
use serde::Deserialize;
use std::convert::TryFrom;
use std::sync::Arc;
use std::sync::RwLock;

#[derive(Deserialize)]
struct Picture {
    picture: String,
}

#[derive(Deserialize, Clone)]
struct PictureQuery {
    #[serde(default = "default_size")]
    size: String,
    #[serde(default)]
    own: bool,
}

fn default_size() -> String {
    "264".to_string()
}

fn retrieve_avatar<T: AsyncCisClientTrait + Clone, L: Loader + Clone>(
    avatar_settings: Data<AvatarSettings>,
    loader: Data<Arc<L>>,
    path: Path<Picture>,
    query: Query<PictureQuery>,
    scope_and_user: ScopeAndUser,
    cis_client: Data<Arc<T>>,
    cache: Data<Arc<RwLock<LruCache<String, String>>>>,
) -> impl Future<Item = HttpResponse, Error = Error> {
    get_uuid(&scope_and_user.user_id, &**cis_client, &*cache, query.own)
        .and_then(move |uuid| {
            retrieve_avatar_from_store(
                &avatar_settings,
                &loader,
                &path.picture,
                query.size.as_str(),
                Some(
                    Display::try_from(scope_and_user.scope.as_str()).unwrap_or_else(|e| {
                        warn!("retriving avatar: {}", e);
                        Display::Public
                    }),
                ),
                uuid,
            )
            .map(|b| {
                HttpResponse::Ok()
                    .encoding(ContentEncoding::Identity)
                    .header("content-type", "image/png")
                    .body(b)
            })
        })
        .map_err(error::ErrorNotFound)
}

fn retrieve_public_avatar<L: Loader + Clone>(
    avatar_settings: Data<AvatarSettings>,
    loader: Data<Arc<L>>,
    path: Path<Picture>,
    query: Query<PictureQuery>,
) -> impl Future<Item = HttpResponse, Error = Error> {
    info!("retrieving public avatar");
    retrieve_avatar_from_store(
        &avatar_settings,
        &loader,
        &path.picture,
        query.size.as_str(),
        Some(Display::Public),
        None,
    )
    .map(|b| {
        HttpResponse::Ok()
            .encoding(ContentEncoding::Identity)
            .header("content-type", "image/png")
            .body(b)
    })
    .map_err(error::ErrorNotFound)
}

pub fn retrieve_app<
    T: AsyncCisClientTrait + Clone + Send + Sync + 'static,
    L: Loader + Clone + Send + Sync + 'static,
>(
    cis_client: Arc<T>,
    avatar_settings: AvatarSettings,
    loader: Arc<L>,
    provider: Provider,
    cache: Arc<RwLock<LruCache<String, String>>>,
) -> impl HttpServiceFactory {
    let scope_middleware = ScopeAndUserAuth { checker: provider };
    web::scope("/get")
        .wrap(
            Cors::new()
                .allowed_methods(vec!["GET"])
                .allowed_headers(vec![http::header::AUTHORIZATION, http::header::ACCEPT])
                .allowed_header(http::header::CONTENT_TYPE)
                .max_age(3600),
        )
        .data(loader)
        .data(avatar_settings)
        .data(cis_client)
        .data(cache)
        .service(
            web::resource("/id/{picture}")
                .guard(guard::fn_guard(|req| {
                    req.headers.contains_key("x-auth-token")
                }))
                .wrap(scope_middleware)
                .route(web::get().to_async(retrieve_avatar::<T, L>)),
        )
        .service(
            web::resource("/id/{picture}").route(web::get().to_async(retrieve_public_avatar::<L>)),
        )
}
