use crate::retrieve::retriever::retrieve_avatar_from_store;
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
use std::convert::TryFrom;
use std::sync::Arc;

#[derive(Deserialize)]
struct Picture {
    picture: String,
}

#[derive(Deserialize)]
struct PictureQuery {
    size: String,
}

fn retrieve_public_avatar<L: Loader + Clone>(
    avatar_settings: Data<AvatarSettings>,
    loader: Data<Arc<L>>,
    path: Path<Picture>,
    query: Option<Query<PictureQuery>>,
) -> impl Future<Item = HttpResponse, Error = Error> {
    info!("retrieving public avatar");
    retrieve_avatar_from_store(
        &avatar_settings,
        &loader,
        &path.picture,
        query.as_ref().map(|q| q.size.as_str()),
        Some(Display::Public),
    )
    .map(|b| {
        HttpResponse::Ok()
            .encoding(ContentEncoding::Identity)
            .header("content-type", "image/png")
            .body(b)
    })
    .map_err(error::ErrorNotFound)
}

fn retrieve_avatar<L: Loader + Clone>(
    avatar_settings: Data<AvatarSettings>,
    loader: Data<Arc<L>>,
    path: Path<Picture>,
    query: Option<Query<PictureQuery>>,
    scope_and_user: ScopeAndUser,
) -> impl Future<Item = HttpResponse, Error = Error> {
    retrieve_avatar_from_store(
        &avatar_settings,
        &loader,
        &path.picture,
        query.as_ref().map(|q| q.size.as_str()),
        Some(
            Display::try_from(scope_and_user.scope.as_str()).unwrap_or_else(|e| {
                warn!("retriving avatar: {}", e);
                Display::Public
            }),
        ),
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
        .service(
            web::resource("/id/{picture}")
                .guard(guard::fn_guard(|req| {
                    req.headers.contains_key("x-auth-token")
                }))
                .wrap(scope_middleware)
                .route(web::get().to_async(retrieve_avatar::<L>)),
        )
        .service(
            web::resource("/id/{picture}").route(web::get().to_async(retrieve_public_avatar::<L>)),
        )
}
