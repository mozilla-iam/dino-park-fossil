use crate::settings::AvatarSettings;
use crate::storage::loader::Loader;
use crate::storage::name::ExternalFileName;
use cis_profile::schema::Display;
use failure::Error;
use futures::future::Either;
use futures::future::IntoFuture;
use futures::Future;
use std::convert::TryFrom;
use std::sync::Arc;

#[derive(Fail, Debug)]
enum RetrieveError {
    #[fail(display = "Picture not found.")]
    NotFound,
}

pub fn retrieve_avatar_from_store(
    settings: &AvatarSettings,
    loader: &Arc<impl Loader>,
    picture: &str,
    size: Option<&str>,
    scope: Option<Display>,
) -> impl Future<Item = Vec<u8>, Error = Error> {
    let size = size.unwrap_or_else(|| "264");
    let internal = match ExternalFileName::from_uri(picture) {
        Ok(external_file_name) => external_file_name.internal,
        Err(e) => {
            warn!("invalid file name: {}", e);
            return Either::B(Err(RetrieveError::NotFound.into()).into_future());
        }
    };
    if let Some(scope) = scope {
        if scope < Display::try_from(internal.display.as_str()).unwrap_or_else(|_| Display::Public)
        {
            return Either::B(Err(RetrieveError::NotFound.into()).into_future());
        }
    }
    Either::A(
        loader
            .load(&internal.to_string(), size, &settings.s3_bucket)
            .map_err(|e| {
                warn!("error loading picture: {}", e);
                RetrieveError::NotFound.into()
            }),
    )
}
