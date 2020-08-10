use crate::send::app::ChangeDisplay;
use crate::send::app::Save;
use crate::send::operations::delete;
use crate::send::operations::delete_many;
use crate::send::operations::rename;
use crate::send::operations::save;
use crate::send::resize::Avatars;
use crate::settings::AvatarSettings;
use crate::storage::loader::Loader;
use crate::storage::name::ExternalFileName;
use crate::storage::name::InternalFileName;
use crate::storage::saver::Saver;
use cis_profile::schema::Display;
use failure::Error;
use log::info;
use log::warn;
use serde::Serialize;
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

pub async fn delete_avatar(
    settings: &AvatarSettings,
    saver: &Arc<impl Saver>,
    uuid: &str,
) -> Result<(), Error> {
    info!("deleting avatar for {}", uuid);
    let internal_file_names = vec![
        InternalFileName::from_uuid_and_display(uuid, &Display::Public).to_string(),
        InternalFileName::from_uuid_and_display(uuid, &Display::Authenticated).to_string(),
        InternalFileName::from_uuid_and_display(uuid, &Display::Vouched).to_string(),
        InternalFileName::from_uuid_and_display(uuid, &Display::Ndaed).to_string(),
        InternalFileName::from_uuid_and_display(uuid, &Display::Staff).to_string(),
        InternalFileName::from_uuid_and_display(uuid, &Display::Private).to_string(),
    ];
    delete_many(&internal_file_names, &settings.s3_bucket, saver).await?;

    Ok(())
}

pub async fn change_display_level(
    settings: &AvatarSettings,
    loader: &Arc<impl Loader>,
    saver: &Arc<impl Saver>,
    uuid: &str,
    change_display: &ChangeDisplay,
) -> Result<PictureUrl, Error> {
    info!("changing display level for {}", uuid);
    let old_file_name = ExternalFileName::from_uri(&change_display.old_url)?;
    let file_name = ExternalFileName::from_uuid_and_display(uuid, &change_display.display);
    let result = PictureUrl {
        url: format!(
            "{}{}{}",
            settings.picture_api_url,
            settings.retrieve_by_id_path,
            &file_name.filename()
        ),
    };
    if old_file_name.internal.uuid_hash != file_name.internal.uuid_hash {
        return Err(SaveError::UuidMismatch.into());
    }
    rename(
        &old_file_name.internal.to_string(),
        &file_name.internal.to_string(),
        &settings.s3_bucket,
        saver,
        loader,
    )
    .await?;
    Ok(result)
}

pub async fn check_resize_store_intermediate(
    settings: &AvatarSettings,
    saver: Arc<impl Saver>,
    loader: Arc<impl Loader>,
    uuid: &str,
    save: Save,
) -> Result<PictureUrl, Error> {
    let buf = loader
        .load(&save.intermediate, "tmp", &settings.s3_bucket)
        .await?;
    check_resize_store(&settings, saver, uuid, buf, &save.display, &save.old_url).await
}

async fn check_resize_store(
    settings: &AvatarSettings,
    saver: Arc<impl Saver>,
    uuid: &str,
    buf: Vec<u8>,
    display: &Display,
    old_url: &Option<String>,
) -> Result<PictureUrl, Error> {
    info!("uploading image for {}", uuid);
    let file_name = ExternalFileName::from_uuid_and_display(uuid, display);
    let avatars = Avatars::new(buf)?;
    let bucket = settings.s3_bucket.clone();
    let result = PictureUrl {
        url: format!(
            "{}{}{}",
            settings.picture_api_url,
            settings.retrieve_by_id_path,
            &file_name.filename()
        ),
    };
    if let Some(old_url) = old_url {
        let old_file_name = ExternalFileName::from_uri(&old_url);
        match old_file_name {
            Ok(name) => {
                delete(&name.internal.to_string(), &settings.s3_bucket, &saver).await?;
            }
            Err(e) => {
                warn!("{} for {}: {}", e, uuid, old_url);
            }
        }
    }
    save(avatars, &file_name.internal.to_string(), &bucket, &saver).await?;
    Ok(result)
}

pub async fn store_intermediate(
    bucket: String,
    saver: Arc<impl Saver>,
    buf: Vec<u8>,
) -> Result<String, Error> {
    saver.save_tmp(&bucket, buf).await
}

#[cfg(test)]
mod test {
    use super::*;
    use failure::format_err;
    use futures::future::BoxFuture;

    struct DummySaver {
        delete: bool,
        save: bool,
    }
    impl Saver for DummySaver {
        fn save(&self, _: &str, _: &str, _: &str, _: Vec<u8>) -> BoxFuture<Result<(), Error>> {
            let ret = match self.save {
                true => Ok(()),
                false => Err(format_err!("doom")),
            };
            Box::pin(async move { ret })
        }
        fn delete(&self, _: &str, _: &str, _: &str) -> BoxFuture<Result<(), Error>> {
            let ret = match self.delete {
                true => Ok(()),
                false => Err(format_err!("doom")),
            };
            Box::pin(async move { ret })
        }
        fn delete_many(&self, _: &[String], _: &str, _: &str) -> BoxFuture<Result<(), Error>> {
            let ret = match self.delete {
                true => Ok(()),
                false => Err(format_err!("doom")),
            };
            Box::pin(async move { ret })
        }
        fn save_tmp(&self, _: &str, _: Vec<u8>) -> BoxFuture<Result<String, Error>> {
            Box::pin(async { Ok(String::from("936DA01F9ABD4d9d80C702AF85C822A8")) })
        }
    }

    #[tokio::test]
    async fn test_check_resize_store_without_old() -> Result<(), Error> {
        let data = include_bytes!("../../tests/data/dino.png");
        let settings = AvatarSettings {
            s3_bucket: String::from("testing"),
            retrieve_by_id_path: String::from("/api/v666"),
            picture_api_url: String::from("https://localhost"),
        };
        let saver = Arc::new(DummySaver {
            delete: true,
            save: true,
        });
        let uuid = "9e697947-2990-4182-b080-533c16af4799";
        check_resize_store(
            &settings,
            saver,
            uuid,
            data.to_vec(),
            &Display::Private,
            &None,
        )
        .await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_check_resize_store_with_old() -> Result<(), Error> {
        let data = include_bytes!("../../tests/data/dino.png");
        let settings = AvatarSettings {
            s3_bucket: String::from("testing"),
            retrieve_by_id_path: String::from("/api/v666"),
            picture_api_url: String::from("https://localhost"),
        };
        let saver = Arc::new(DummySaver {
            delete: true,
            save: true,
        });
        let uuid = "9e697947-2990-4182-b080-533c16af4799";
        let old_url = Some(String::from(
                "MmU5ODFiODZkNWY3N2Y1NDY2ZWM1NmUyYjQwM2RlYWUyOTI3MGYwMDllOGFmZGE1ODNjZjEyNzQ3YjQ0NzQyNiNzdGFmZiMxNTU0MDQ1OTgz.png"));
        check_resize_store(
            &settings,
            saver,
            uuid,
            data.to_vec(),
            &Display::Private,
            &old_url,
        )
        .await?;
        Ok(())
    }
}
