use crate::name::name_from_uuid;
use crate::send::app::Avatar;
use crate::send::resize::png_from_data_uri;
use crate::send::resize::save;
use crate::send::resize::Avatars;
use crate::send::saver::Saver;
use crate::settings::AvatarSettings;
use failure::Error;
use serde_json::json;
use serde_json::Value;

pub fn check_resize_store(
    settings: &AvatarSettings,
    saver: &impl Saver,
    uuid: &str,
    avatar: &Avatar,
) -> Result<Value, Error> {
    info!("uploading image for {}", uuid);
    let avatars = Avatars::new(&png_from_data_uri(&avatar.data_uri)?)?;
    let name = name_from_uuid(uuid);
    save(avatars, &name, &settings.s3_bucket, saver)?;
    Ok(json!({
        "url": format!("{}{}.png", settings.retrieve_by_id_path, name)
    }))
}
