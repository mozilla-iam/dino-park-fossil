use crate::scale::convert::handle_multipart_item;
use actix_multipart::Multipart;
use actix_web::dev::HttpServiceFactory;

use actix_cors::Cors;
use actix_web::http;
use actix_web::http::ContentEncoding;
use actix_web::middleware::BodyEncoding;
use actix_web::web;
use actix_web::web::Path;
use actix_web::HttpResponse;
use failure::Error;
use futures::Future;

pub fn echo(
    size: Path<u32>,
    multipart: Multipart,
) -> impl Future<Item = HttpResponse, Error = Error> {
    handle_multipart_item(*size, multipart)
        .map(|b| {
            HttpResponse::Ok()
                .encoding(ContentEncoding::Identity)
                .header("content-type", "image/png")
                .body(b)
        })
        .map_err(Into::into)
}
pub fn scale_app() -> impl HttpServiceFactory {
    web::scope("/scale")
        .wrap(
            Cors::new()
                .allowed_methods(vec!["POST"])
                .allowed_headers(vec![http::header::AUTHORIZATION, http::header::ACCEPT])
                .allowed_header(http::header::CONTENT_TYPE)
                .max_age(3600),
        )
        .service(web::resource("/{size}").route(web::post().to_async(echo)))
}
