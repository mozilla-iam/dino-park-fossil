use crate::scale::convert::handle_multipart_item;
use actix_web::error;
use actix_web::http;
use actix_web::http::ContentEncoding;
use actix_web::middleware::cors::Cors;
use actix_web::App;
use actix_web::FromRequest;
use actix_web::FutureResponse;
use actix_web::HttpMessage;
use actix_web::HttpRequest;
use actix_web::HttpResponse;
use actix_web::Path;
use futures::Future;
use futures::Stream;

pub fn echo(req: HttpRequest) -> FutureResponse<HttpResponse> {
    let size: u32 = Path::<u32>::extract(&req)
        .map(|s| *s)
        .unwrap_or_else(|_| 512);
    Box::new(
        req.multipart()
            .map(move |p| handle_multipart_item(size, p))
            .flatten()
            .collect()
            .map(|mut b| {
                HttpResponse::Ok()
                    .content_encoding(ContentEncoding::Identity)
                    .header("content-type", "image/png")
                    .body(b.pop().unwrap())
            })
            .map_err(|e| {
                println!("failed: {}", e);
                error::ErrorBadRequest(e)
            }),
    )
}
pub fn scale_app() -> App {
    App::new().prefix("/avatar/scale").configure(|app| {
        Cors::for_app(app)
            .allowed_methods(vec!["POST"])
            .allowed_headers(vec![http::header::AUTHORIZATION, http::header::ACCEPT])
            .allowed_header(http::header::CONTENT_TYPE)
            .max_age(3600)
            .resource("/{size}", |r| r.method(http::Method::POST).with(echo))
            .register()
    })
}
