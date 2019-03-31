use crate::scope::Scope;
use crate::settings::AvatarSettings;
use crate::storage::loader::Loader;
use crate::storage::name::ExternalFileName;
use cis_client::client::CisClientTrait;
use cis_client::client::GetBy;
use failure::Error;

#[derive(Debug, Fail)]
pub enum RetrieverError {
    #[fail(display = "no picture for username")]
    NoPictureForUsername,
}

pub fn check_and_retrieve_avatar_by_username_from_store(
    cis_client: &impl CisClientTrait,
    settings: &AvatarSettings,
    loader: &impl Loader,
    username: &str,
    scope: &Option<Scope>,
) -> Result<Vec<u8>, Error> {
    info!("{} → {:?}", username, scope);
    let filter = scope.as_ref().map(|s| s.scope.as_str());
    let profile = cis_client.get_user_by(username, &GetBy::PrimaryUsername, filter)?;
    if let Some(picture) = profile.picture.value {
        let name = ExternalFileName::from_uri(&picture)?.internal.to_string();
        let buf = loader.load(&name, "264", &settings.s3_bucket)?;
        return Ok(buf);
    }
    Err(RetrieverError::NoPictureForUsername.into())
}

pub fn retrieve_avatar_from_store(
    settings: &AvatarSettings,
    loader: &impl Loader,
    picture: &str,
    size: Option<&str>,
) -> Result<Vec<u8>, Error> {
    let name = ExternalFileName::from_uri(picture)?.internal.to_string();
    let size = size.unwrap_or_else(|| "264");
    loader.load(&name, size, &settings.s3_bucket)
}
