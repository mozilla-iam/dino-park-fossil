use crate::retrieve::retriever::retrieve_avatar_from_store;
use crate::retrieve::uuid::get_uuid;
use crate::settings::AvatarSettings;
use crate::storage::loader::Loader;
use actix_cors::Cors;
use actix_web::dev::BodyEncoding;
use actix_web::dev::HttpServiceFactory;
use actix_web::error;
use actix_web::http;
use actix_web::http::ContentEncoding;
use actix_web::web;
use actix_web::web::Data;
use actix_web::web::Path;
use actix_web::web::Query;
use actix_web::Error;
use actix_web::HttpResponse;
use cis_client::AsyncCisClientTrait;
use cis_profile::schema::Display;
use dino_park_gate::scope::ScopeAndUser;
use dino_park_gate::scope::ScopeAndUserAuth;
use dino_park_trust::Trust;
use lru_time_cache::LruCache;
use serde::Deserialize;
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

async fn retrieve_avatar<T: AsyncCisClientTrait + Clone, L: Loader>(
    avatar_settings: Data<AvatarSettings>,
    loader: Data<L>,
    path: Path<Picture>,
    query: Query<PictureQuery>,
    scope_and_user: ScopeAndUser,
    cis_client: Data<T>,
    cache: Data<RwLock<LruCache<String, String>>>,
) -> Result<HttpResponse, Error> {
    let uuid = if scope_and_user.scope != Trust::Public {
        let cis_client = cis_client.into_inner();
        get_uuid(&scope_and_user.user_id, &*cis_client, &*cache, query.own)
            .await
            .map_err(error::ErrorNotFound)?
    } else {
        None
    };
    let b = retrieve_avatar_from_store(
        &avatar_settings,
        &loader.into_inner(),
        &path.picture,
        query.size.as_str(),
        Some(Display::from(scope_and_user.scope)),
        uuid,
    )
    .await
    .map_err(error::ErrorNotFound)?;
    Ok(HttpResponse::Ok()
        .encoding(ContentEncoding::Identity)
        .header("content-type", "image/png")
        .body(b))
}

pub fn retrieve_app<
    T: AsyncCisClientTrait + Clone + Send + Sync + 'static,
    L: Loader + Send + Sync + 'static,
>(
    middleware: ScopeAndUserAuth,
) -> impl HttpServiceFactory {
    web::scope("/get")
        .wrap(
            Cors::new()
                .allowed_methods(vec!["GET"])
                .allowed_headers(vec![http::header::AUTHORIZATION, http::header::ACCEPT])
                .allowed_header(http::header::CONTENT_TYPE)
                .max_age(3600)
                .finish(),
        )
        .wrap(middleware)
        .service(web::resource("/id/{picture}").route(web::get().to(retrieve_avatar::<T, L>)))
}
