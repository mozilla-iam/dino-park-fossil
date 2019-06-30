use actix_web::dev::Payload;
use actix_web::error;
use actix_web::Error;
use actix_web::FromRequest;
use actix_web::HttpRequest;
use actix_web::Result;

#[derive(Deserialize, Debug)]
pub struct Scope {
    pub scope: String,
}

impl FromRequest for Scope {
    type Config = ();
    type Future = Result<Self, Error>;
    type Error = Error;

    #[inline]
    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        req.headers()
            .get("scope")
            .and_then(|h| h.to_str().ok())
            .map(|h| Scope {
                scope: h.to_owned(),
            })
            .ok_or_else(|| error::ErrorForbidden("no scope"))
    }
}
