use crate::settings::AvatarSettings;
use crate::storage::loader::Loader;
use crate::storage::name::uuid_hash;
use crate::storage::name::ExternalFileName;
use cis_profile::schema::Display;
use failure::Error;
use log::warn;
use std::convert::TryFrom;
use std::sync::Arc;

#[derive(Fail, Debug, PartialEq)]
enum RetrieveError {
    #[fail(display = "Picture not found.")]
    NotFound,
}

pub async fn retrieve_avatar_from_store(
    settings: &AvatarSettings,
    loader: &Arc<impl Loader>,
    picture: &str,
    size: &str,
    scope: Option<Display>,
    uuid: Option<String>,
) -> Result<Vec<u8>, Error> {
    let internal = match ExternalFileName::from_uri(picture) {
        Ok(external_file_name) => external_file_name.internal,
        Err(e) => {
            warn!("invalid file name: {}", e);
            return Err(RetrieveError::NotFound.into());
        }
    };
    let scope = match uuid.map(|uuid| internal.uuid_hash == uuid_hash(&uuid)) {
        Some(true) => Some(Display::Private),
        _ => scope,
    };
    if let Some(scope) = scope {
        if scope < Display::try_from(internal.display.as_str()).unwrap_or_else(|_| Display::Public)
        {
            return Err(RetrieveError::NotFound.into());
        }
    }
    let is_528 = size == "528";
    let internal_s = internal.to_string();
    match loader.load(&internal_s, size, &settings.s3_bucket).await {
        Ok(data) => Ok(data),
        Err(_) if is_528 => loader.load(&internal_s, "264", &settings.s3_bucket).await,
        Err(e) => Err(e),
    }
    .map_err(|e| {
        warn!("error loading picture: {}", e);
        RetrieveError::NotFound.into()
    })
}

#[cfg(test)]
mod test {
    use super::*;
    use failure::format_err;
    use futures::future::BoxFuture;

    struct DummyLoader {
        retrieve_528: bool,
        name: String,
    }

    impl Loader for DummyLoader {
        fn load(&self, name: &str, size: &str, _: &str) -> BoxFuture<Result<Vec<u8>, Error>> {
            let ret = if name != self.name {
                Err(format_err!("404"))
            } else {
                match size {
                    "528" => {
                        if self.retrieve_528 {
                            Ok(vec![0; 528])
                        } else {
                            Err(format_err!("no 528"))
                        }
                    }
                    "264" => Ok(vec![0; 264]),
                    _ => Err(format_err!("doom")),
                }
            };
            Box::pin(async move { ret })
        }
    }

    #[tokio::test]
    async fn test_264_retrieved_when_528_fails() -> Result<(), Error> {
        let uuid = "9e697947-2990-4182-b080-533c16af4799";
        let display = "public";

        let settings = AvatarSettings {
            s3_bucket: String::from("testing"),
            retrieve_by_id_path: String::from("/api/v666"),
            picture_api_url: String::from("https://localhost"),
        };
        let picture = ExternalFileName::from_uuid_and_display(uuid, display);
        let size = String::from("528");

        let loader = Arc::new(DummyLoader {
            retrieve_528: false,
            name: picture.internal.to_string(),
        });

        let avatar =
            retrieve_avatar_from_store(&settings, &loader, &picture.filename(), &size, None, None)
                .await?;

        assert_eq!(avatar.len(), 264);
        Ok(())
    }

    #[tokio::test]
    async fn test_528_retrieved_when_available() -> Result<(), Error> {
        let uuid = "9e697947-2990-4182-b080-533c16af4799";
        let display = "public";

        let settings = AvatarSettings {
            s3_bucket: String::from("testing"),
            retrieve_by_id_path: String::from("/api/v666"),
            picture_api_url: String::from("https://localhost"),
        };
        let picture = ExternalFileName::from_uuid_and_display(uuid, display);
        let loader = Arc::new(DummyLoader {
            retrieve_528: true,
            name: picture.internal.to_string(),
        });
        let size = String::from("528");

        let avatar =
            retrieve_avatar_from_store(&settings, &loader, &picture.filename(), &size, None, None)
                .await?;

        assert_eq!(avatar.len(), 528);
        Ok(())
    }

    #[tokio::test]
    async fn test_own_fails_for_wrong_uuid() -> Result<(), Error> {
        let uuid = "9e697947-2990-4182-b080-533c16af4799";
        let wrong_uuid = "9e697947-2990-4182-b080-533c16af4790";
        let display = "staff";

        let settings = AvatarSettings {
            s3_bucket: String::from("testing"),
            retrieve_by_id_path: String::from("/api/v666"),
            picture_api_url: String::from("https://localhost"),
        };
        let picture = ExternalFileName::from_uuid_and_display(uuid, display);
        let loader = Arc::new(DummyLoader {
            retrieve_528: true,
            name: picture.internal.to_string(),
        });
        let size = String::from("528");

        let res = retrieve_avatar_from_store(
            &settings,
            &loader,
            &picture.filename(),
            &size,
            Some(Display::Public),
            Some(wrong_uuid.to_owned()),
        )
        .await;

        assert_eq!(
            res.err().unwrap().downcast::<RetrieveError>()?,
            RetrieveError::NotFound
        );
        Ok(())
    }

    #[tokio::test]
    async fn test_own_works() -> Result<(), Error> {
        let uuid = "9e697947-2990-4182-b080-533c16af4799";
        let display = "staff";

        let settings = AvatarSettings {
            s3_bucket: String::from("testing"),
            retrieve_by_id_path: String::from("/api/v666"),
            picture_api_url: String::from("https://localhost"),
        };
        let picture = ExternalFileName::from_uuid_and_display(uuid, display);
        let loader = Arc::new(DummyLoader {
            retrieve_528: true,
            name: picture.internal.to_string(),
        });
        let size = String::from("528");

        let avatar = retrieve_avatar_from_store(
            &settings,
            &loader,
            &picture.filename(),
            &size,
            Some(Display::Public),
            Some(uuid.to_owned()),
        )
        .await?;

        assert_eq!(avatar.len(), 528);
        Ok(())
    }
}
