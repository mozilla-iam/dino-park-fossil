use crate::scope::Scope;
use crate::settings::AvatarSettings;
use crate::storage::loader::Loader;
use crate::storage::name::ExternalFileName;
use cis_client::getby::GetBy;
use cis_client::AsyncCisClientTrait;
use cis_profile::schema::Display;
use failure::Error;
use futures::future::Either;
use futures::future::IntoFuture;
use futures::Future;
use std::sync::Arc;
use std::convert::TryFrom;

const DINO_1024: &[u8] = include_bytes!("../data/dino_1024.png");
const DINO_528: &[u8] = include_bytes!("../data/dino_528.png");
const DINO_512: &[u8] = include_bytes!("../data/dino_512.png");
const DINO_264: &[u8] = include_bytes!("../data/dino_264.png");
const DINO_200: &[u8] = include_bytes!("../data/dino_200.png");
const DINO_100: &[u8] = include_bytes!("../data/dino_100.png");
const DINO_80: &[u8] = include_bytes!("../data/dino_80.png");
const DINO_40: &[u8] = include_bytes!("../data/dino_40.png");

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

fn get_dino(size: &str) -> Vec<u8> {
    match size {
        "40" => DINO_40,
        "80" => DINO_80,
        "100" => DINO_100,
        "200" => DINO_200,
        "264" => DINO_264,
        "512" => DINO_512,
        "528" => DINO_528,
        "1024" => DINO_1024,
        _ => DINO_512,
    }.into()
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
        Err(e) => return Either::B(Err(e).into_future()),
    };
    if let Some(scope) = scope {
        if scope < Display::try_from(internal.display.as_str()).unwrap_or_else(|_| Display::Public) {
            return Either::B(Ok(get_dino(size)).into_future());
        }
    }
    Either::A(loader.load(&internal.to_string(), size, &settings.s3_bucket))
}
