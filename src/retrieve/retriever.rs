use crate::settings::AvatarSettings;
use crate::storage::loader::Loader;
use crate::storage::name::uuid_hash;
use crate::storage::name::ExternalFileName;
use cis_profile::schema::Display;
use failure::Error;
use futures::future::Either;
use futures::future::IntoFuture;
use futures::Future;
use log::warn;
use std::convert::TryFrom;
use std::sync::Arc;

#[derive(Fail, Debug, PartialEq)]
enum RetrieveError {
    #[fail(display = "Picture not found.")]
    NotFound,
}

pub fn retrieve_avatar_from_store(
    settings: &AvatarSettings,
    loader: &Arc<impl Loader>,
    picture: &str,
    size: &str,
    scope: Option<Display>,
    uuid: Option<String>,
) -> impl Future<Item = Vec<u8>, Error = Error> {
    let internal = match ExternalFileName::from_uri(picture) {
        Ok(external_file_name) => external_file_name.internal,
        Err(e) => {
            warn!("invalid file name: {}", e);
            return Either::B(Err(RetrieveError::NotFound.into()).into_future());
        }
    };
    let scope = match uuid.map(|uuid| internal.uuid_hash == uuid_hash(&uuid)) {
        Some(true) => Some(Display::Private),
        _ => scope,
    };
    if let Some(scope) = scope {
        if scope < Display::try_from(internal.display.as_str()).unwrap_or_else(|_| Display::Public)
        {
            return Either::B(Err(RetrieveError::NotFound.into()).into_future());
        }
    }
    let is_528 = size == "528";
    let fallback_loader = Arc::clone(loader);
    let fallback_internal = internal.to_string();
    let fallback_bucket = settings.s3_bucket.clone();
    Either::A(
        loader
            .load(&internal.to_string(), size, &settings.s3_bucket)
            .or_else(move |e| {
                if is_528 {
                    fallback_loader.load(&fallback_internal, "264", &fallback_bucket)
                } else {
                    Box::new(Err(e).into_future())
                }
            })
            .map_err(|e| {
                warn!("error loading picture: {}", e);
                RetrieveError::NotFound.into()
            }),
    )
}

#[cfg(test)]
mod test {
    use super::*;
    use failure::format_err;

    struct DummyLoader {
        retrieve_528: bool,
        name: String,
    }

    impl Loader for DummyLoader {
        fn load(
            &self,
            name: &str,
            size: &str,
            _: &str,
        ) -> Box<dyn Future<Item = Vec<u8>, Error = Error>> {
            if name != self.name {
                return Box::new(Err(format_err!("404")).into_future());
            }
            let ret = match size {
                "528" => {
                    if self.retrieve_528 {
                        Ok(vec![0; 528])
                    } else {
                        Err(format_err!("no 528"))
                    }
                }
                "264" => Ok(vec![0; 264]),
                _ => Err(format_err!("doom")),
            };
            Box::new(ret.into_future())
        }
    }

    #[test]
    fn test_264_retrieved_when_528_fails() -> Result<(), Error> {
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
                .wait()?;

        assert_eq!(avatar.len(), 264);
        Ok(())
    }

    #[test]
    fn test_528_retrieved_when_available() -> Result<(), Error> {
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
                .wait()?;

        assert_eq!(avatar.len(), 528);
        Ok(())
    }

    #[test]
    fn test_own_fails_for_wrong_uuid() -> Result<(), Error> {
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
        .wait();

        assert_eq!(
            res.err().unwrap().downcast::<RetrieveError>()?,
            RetrieveError::NotFound
        );
        Ok(())
    }

    #[test]
    fn test_own_works() -> Result<(), Error> {
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
        .wait()?;

        assert_eq!(avatar.len(), 528);
        Ok(())
    }
}
