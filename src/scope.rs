use actix_web::error;
use actix_web::FromRequest;
use actix_web::HttpRequest;
use actix_web::Result;

#[derive(Deserialize, Debug)]
pub struct Scope {
    pub scope: String,
}

impl<S> FromRequest<S> for Scope {
    type Config = ();
    type Result = Result<Scope, error::Error>;

    #[inline]
    fn from_request(req: &HttpRequest<S>, _cfg: &Self::Config) -> Self::Result {
        req.headers()
            .get("scope")
            .and_then(|h| h.to_str().ok())
            .map(|h| Scope {
                scope: h.to_owned(),
            })
            .ok_or_else(|| error::ErrorForbidden("no scope"))
    }
}
