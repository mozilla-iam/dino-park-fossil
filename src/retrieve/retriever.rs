use crate::name::uuid_from_name;
use crate::retrieve::loader::Loader;
use crate::scope::Scope;
use crate::settings::AvatarSettings;
use cis_client::client::CisClientTrait;
use cis_client::client::GetBy;
use failure::Error;

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
    let buf = loader.load(&profile.uuid.value.unwrap(), "264", &settings.s3_bucket)?;

    Ok(buf)
}

pub fn retrieve_avatar_from_store(
    settings: &AvatarSettings,
    loader: &impl Loader,
    picture: &str,
    size: Option<&str>,
) -> Result<Vec<u8>, Error> {
    let id = match (picture.rfind('/'), picture.rfind('.')) {
        (Some(start), Some(end)) => &picture[start..end],
        (Some(start), None) => &picture[start..],
        (None, Some(end)) => &picture[..end],
        _ => picture,
    };
    let uuid = uuid_from_name(id)?;
    let size = size.unwrap_or_else(|| "264");
    loader.load(&uuid, size, &settings.s3_bucket)
}
