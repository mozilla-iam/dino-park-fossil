use crate::scope::Scope;
use crate::settings::AvatarSettings;
use crate::storage::loader::Loader;
use crate::storage::name::ExternalFileName;
use cis_client::getby::GetBy;
use cis_client::AsyncCisClientTrait;
use failure::Error;
use futures::future::Either;
use futures::future::IntoFuture;
use futures::Future;
use std::sync::Arc;

#[derive(Debug, Fail)]
pub enum RetrieverError {
    #[fail(display = "no picture for username")]
    NoPictureForUsername,
}

pub fn check_and_retrieve_avatar_by_username_from_store(
    cis_client: &Arc<impl AsyncCisClientTrait>,
    settings: &AvatarSettings,
    loader: &Arc<impl Loader>,
    username: &str,
    scope: &Option<Scope>,
) -> impl Future<Item = Vec<u8>, Error = Error> {
    info!("{} → {:?}", username, scope);
    let filter = scope.as_ref().map(|s| s.scope.as_str());
    let bucket = settings.s3_bucket.clone();
    let loader = Arc::clone(loader);
    cis_client
        .get_user_by(username, &GetBy::PrimaryUsername, filter)
        .and_then(move |profile| {
            if let Some(picture) = profile.picture.value {
                match ExternalFileName::from_uri(&picture) {
                    Ok(external_file_name) => {
                        return Either::A(loader.load(
                            &external_file_name.internal.to_string(),
                            "264",
                            &bucket,
                        ))
                    }
                    Err(e) => return Either::B(Err(e).into_future()),
                }
            }
            Either::B(Err(RetrieverError::NoPictureForUsername.into()).into_future())
        })
}

pub fn retrieve_avatar_from_store(
    settings: &AvatarSettings,
    loader: &Arc<impl Loader>,
    picture: &str,
    size: Option<&str>,
) -> impl Future<Item = Vec<u8>, Error = Error> {
    let size = size.unwrap_or_else(|| "264");
    let name = match ExternalFileName::from_uri(picture) {
        Ok(external_file_name) => external_file_name.internal.to_string(),
        Err(e) => return Either::B(Err(e).into_future()),
    };
    Either::A(loader.load(&name, size, &settings.s3_bucket))
}
