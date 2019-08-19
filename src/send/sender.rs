use crate::send::app::Avatar;
use crate::send::app::ChangeDisplay;
use crate::send::app::Save;
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
use futures::future::Either;
use futures::Future;
use futures::IntoFuture;
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
) -> impl Future<Item = PictureUrl, Error = Error> {
    info!("changing display level for {}", uuid);
    let old_file_name = match ExternalFileName::from_uri(&change_display.old_url) {
        Ok(old_file_name) => old_file_name,
        Err(e) => return Either::B(Err(e).into_future()),
    };
    let file_name = ExternalFileName::from_uuid_and_display(uuid, &change_display.display);
    let result = PictureUrl {
        url: format!("{}{}", settings.retrieve_by_id_path, &file_name.filename()),
    };
    if old_file_name.internal.uuid_hash != file_name.internal.uuid_hash {
        return Either::B(Err(SaveError::UuidMismatch.into()).into_future());
    }
    Either::A(
        rename(
            &old_file_name.internal.to_string(),
            &file_name.internal.to_string(),
            &settings.s3_bucket,
            saver,
            loader,
        )
        .map(move |_| result),
    )
}

pub fn check_resize_store_data_uri(
    settings: &AvatarSettings,
    saver: &Arc<impl Saver>,
    uuid: &str,
    avatar: Avatar,
) -> impl Future<Item = PictureUrl, Error = Error> {
    let buf = match png_from_data_uri(&avatar.data_uri) {
        Ok(buf) => buf,
        Err(e) => return Either::B(Err(e).into_future()),
    };
    Either::A(check_resize_store(
        settings,
        saver,
        uuid,
        buf,
        &avatar.display,
        &avatar.old_url,
    ))
}

pub fn check_resize_store_intermediate(
    settings: &AvatarSettings,
    saver: &Arc<impl Saver>,
    loader: &Arc<impl Loader>,
    uuid: &str,
    save: Save,
) -> impl Future<Item = PictureUrl, Error = Error> {
    let settings = settings.clone();
    let saver = Arc::clone(saver);
    let loader = Arc::clone(loader);
    let uuid = uuid.to_owned();
    loader
        .load(&save.intermediate, "tmp", &settings.s3_bucket)
        .and_then(move |buf| {
            check_resize_store(&settings, &saver, &uuid, buf, &save.display, &save.old_url)
        })
}

fn check_resize_store(
    settings: &AvatarSettings,
    saver: &Arc<impl Saver>,
    uuid: &str,
    buf: Vec<u8>,
    display: &str,
    old_url: &Option<String>,
) -> impl Future<Item = PictureUrl, Error = Error> {
    info!("uploading image for {}", uuid);
    let file_name = ExternalFileName::from_uuid_and_display(uuid, display);
    let avatars = match Avatars::new(buf) {
        Ok(avatars) => avatars,
        Err(e) => return Either::B(Err(e).into_future()),
    };
    let saver = Arc::clone(saver);
    let bucket = settings.s3_bucket.clone();
    let result = PictureUrl {
        url: format!("{}{}", settings.retrieve_by_id_path, &file_name.filename()),
    };
    Either::A(
        {
            if let Some(old_url) = old_url {
                let old_file_name = ExternalFileName::from_uri(&old_url);
                match old_file_name {
                    Ok(name) => Either::A(delete(
                        &name.internal.to_string(),
                        &settings.s3_bucket,
                        &saver,
                    )),
                    Err(e) => {
                        warn!("{} for {}: {}", e, uuid, old_url);
                        Either::B(Ok(()).into_future())
                    }
                }
            } else {
                Either::B(Ok(()).into_future())
            }
        }
        .and_then(move |_| save(avatars, &file_name.internal.to_string(), &bucket, &saver))
        .map(|_| result),
    )
}

pub fn store_intermediate(
    bucket: String,
    saver: Arc<impl Saver>,
    buf: Vec<u8>,
) -> impl Future<Item = String, Error = Error> {
    saver.save_tmp(&bucket, buf)
}

#[cfg(test)]
mod test {
    use super::*;

    struct DummySaver {
        delete: bool,
        save: bool,
    }
    impl Saver for DummySaver {
        fn save(
            &self,
            _: &str,
            _: &str,
            _: &str,
            _: Vec<u8>,
        ) -> Box<dyn Future<Item = (), Error = Error>> {
            let ret = match self.save {
                true => Ok(()),
                false => Err(format_err!("doom")),
            };
            Box::new(ret.into_future())
        }
        fn delete(&self, _: &str, _: &str, _: &str) -> Box<dyn Future<Item = (), Error = Error>> {
            let ret = match self.delete {
                true => Ok(()),
                false => Err(format_err!("doom")),
            };
            Box::new(ret.into_future())
        }
        fn save_tmp(&self, _: &str, _: Vec<u8>) -> Box<dyn Future<Item = String, Error = Error>> {
            Box::new(Ok(String::from("936DA01F9ABD4d9d80C702AF85C822A8")).into_future())
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
        check_resize_store_data_uri(&settings, &saver, uuid, avatar).wait()?;
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
        check_resize_store_data_uri(&settings, &saver, uuid, avatar).wait()?;
        Ok(())
    }
}
