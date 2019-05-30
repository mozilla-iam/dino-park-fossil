use crate::send::app::Avatar;
use crate::send::app::ChangeDisplay;
use crate::send::operations::delete;
use crate::send::operations::rename;
use crate::send::operations::save;
use crate::send::resize::png_from_data_uri;
use crate::send::resize::Avatars;
use crate::settings::AvatarSettings;
use crate::storage::loader::Loader;
use crate::storage::name::ExternalFileName;
use crate::storage::saver::Saver;
use failure::Error;
use std::sync::Arc;

#[derive(Debug, Fail)]
pub enum SaveError {
    #[fail(display = "uuid mismatch")]
    UuidMismatch,
}

#[derive(Serialize)]
pub struct PictureUrl {
    pub url: String,
}

pub fn change_display_level(
    settings: &AvatarSettings,
    loader: &Arc<impl Loader>,
    saver: &Arc<impl Saver>,
    uuid: &str,
    change_display: &ChangeDisplay,
) -> Result<PictureUrl, Error> {
    info!("changing display level for {}", uuid);
    let old_file_name = ExternalFileName::from_uri(&change_display.old_url)?;
    let file_name = ExternalFileName::from_uuid_and_display(uuid, &change_display.display);
    if old_file_name.internal.uuid_hash != file_name.internal.uuid_hash {
        return Err(SaveError::UuidMismatch.into());
    }
    rename(
        &old_file_name.internal.to_string(),
        &file_name.internal.to_string(),
        &settings.s3_bucket,
        saver,
        loader,
    )?;
    Ok(PictureUrl {
        url: format!("{}{}", settings.retrieve_by_id_path, &file_name.filename()),
    })
}

pub fn check_resize_store(
    settings: &AvatarSettings,
    saver: &Arc<impl Saver>,
    uuid: &str,
    avatar: &Avatar,
) -> Result<PictureUrl, Error> {
    info!("uploading image for {}", uuid);
    let avatars = Avatars::new(&png_from_data_uri(&avatar.data_uri)?)?;
    let file_name = ExternalFileName::from_uuid_and_display(uuid, &avatar.display);
    if let Some(old_url) = &avatar.old_url {
        let old_file_name = ExternalFileName::from_uri(&old_url);
        match old_file_name {
            Ok(name) => delete(&name.internal.to_string(), &settings.s3_bucket, saver)?,
            Err(e) => warn!("{} for {}: {}", e, uuid, old_url),
        }
    }
    save(
        avatars,
        &file_name.internal.to_string(),
        &settings.s3_bucket,
        saver,
    )?;
    Ok(PictureUrl {
        url: format!("{}{}", settings.retrieve_by_id_path, &file_name.filename()),
    })
}

#[cfg(test)]
mod test {
    use super::*;

    struct DummySaver {
        delete: bool,
        save: bool,
    }
    impl Saver for DummySaver {
        fn save(&self, _: &str, _: &str, _: &str, _: Vec<u8>) -> Result<(), Error> {
            match self.save {
                true => Ok(()),
                false => Err(format_err!("doom")),
            }
        }
        fn delete(&self, _: &str, _: &str, _: &str) -> Result<(), Error> {
            match self.delete {
                true => Ok(()),
                false => Err(format_err!("doom")),
            }
        }
    }

    #[test]
    fn test_check_resize_store_without_old() -> Result<(), Error> {
        let data = include_str!("../../tests/data/dino.data");
        let settings = AvatarSettings {
            s3_bucket: String::from("testing"),
            retrieve_by_id_path: String::from("/api/v666"),
        };
        let saver = Arc::new(DummySaver {
            delete: true,
            save: true,
        });
        let uuid = "9e697947-2990-4182-b080-533c16af4799";
        let avatar = Avatar {
            data_uri: String::from(data),
            display: String::from("private"),
            old_url: None,
        };
        check_resize_store(&settings, &saver, uuid, &avatar)?;
        Ok(())
    }

    #[test]
    fn test_check_resize_store_with_old() -> Result<(), Error> {
        let data = include_str!("../../tests/data/dino.data");
        let settings = AvatarSettings {
            s3_bucket: String::from("testing"),
            retrieve_by_id_path: String::from("/api/v666"),
        };
        let saver = Arc::new(DummySaver {
            delete: true,
            save: true,
        });
        let uuid = "9e697947-2990-4182-b080-533c16af4799";
        let avatar = Avatar {
            data_uri: String::from(data),
            display: String::from("private"),
            old_url: Some(String::from(
                "MmU5ODFiODZkNWY3N2Y1NDY2ZWM1NmUyYjQwM2RlYWUyOTI3MGYwMDllOGFmZGE1ODNjZjEyNzQ3YjQ0NzQyNiNzdGFmZiMxNTU0MDQ1OTgz.png",
            )),
        };
        check_resize_store(&settings, &saver, uuid, &avatar)?;
        Ok(())
    }
}
