use crate::retrieve::retriever::check_and_retrieve_avatar_by_username_from_store;
use crate::retrieve::retriever::retrieve_avatar_from_store;
use crate::scope::Scope;
use crate::settings::AvatarSettings;
use crate::storage::loader::Loader;
use actix_cors::Cors;
use actix_web::dev::HttpServiceFactory;
use actix_web::error;
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
use futures::Future;
use std::sync::Arc;

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

fn retrieve_avatar_by_username<T: AsyncCisClientTrait + Clone, L: Loader + Clone>(
    cis_client: Data<Arc<T>>,
    avatar_settings: Data<AvatarSettings>,
    loader: Data<Arc<L>>,
    path: Path<Username>,
    scope: Option<Scope>,
) -> impl Future<Item = HttpResponse, Error = Error> {
    check_and_retrieve_avatar_by_username_from_store(
        &cis_client,
        &avatar_settings,
        &loader,
        &path.username,
        &scope,
    )
    .map(|b| {
        HttpResponse::Ok()
            .encoding(ContentEncoding::Identity)
            .header("content-type", "image/png")
            .body(b)
    })
    .map_err(error::ErrorBadRequest)
}

fn retrieve_avatar<L: Loader + Clone>(
    avatar_settings: Data<AvatarSettings>,
    loader: Data<Arc<L>>,
    path: Path<Picture>,
    query: Option<Query<PictureQuery>>,
) -> impl Future<Item = HttpResponse, Error = Error> {
    retrieve_avatar_from_store(
        &avatar_settings,
        &loader,
        &path.picture,
        query.as_ref().map(|q| q.size.as_str()),
    )
    .map(|b| {
        HttpResponse::Ok()
            .encoding(ContentEncoding::Identity)
            .header("content-type", "image/png")
            .body(b)
    })
    .map_err(error::ErrorBadRequest)
}

pub fn retrieve_app<
    T: AsyncCisClientTrait + Clone + Send + Sync + 'static,
    L: Loader + Clone + Send + Sync + 'static,
>(
    cis_client: Arc<T>,
    avatar_settings: AvatarSettings,
    loader: Arc<L>,
) -> impl HttpServiceFactory {
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
        .service(web::resource("/id/{picture}").route(web::get().to_async(retrieve_avatar::<L>)))
        .service(
            web::resource("/{username}")
                .route(web::get().to_async(retrieve_avatar_by_username::<T, L>)),
        )
}
