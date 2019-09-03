use crate::settings::AvatarSettings;
use crate::storage::loader::Loader;
use crate::storage::name::ExternalFileName;
use cis_profile::schema::Display;
use failure::Error;
use futures::future::Either;
use futures::future::IntoFuture;
use futures::Future;
use log::warn;
use std::convert::TryFrom;
use std::sync::Arc;

#[derive(Fail, Debug)]
enum RetrieveError {
    #[fail(display = "Picture not found.")]
    NotFound,
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
        Err(e) => {
            warn!("invalid file name: {}", e);
            return Either::B(Err(RetrieveError::NotFound.into()).into_future());
        }
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
    }

    impl Loader for DummyLoader {
        fn load(
            &self,
            _: &str,
            size: &str,
            _: &str,
        ) -> Box<dyn Future<Item = Vec<u8>, Error = Error>> {
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
        let loader = Arc::new(DummyLoader {
            retrieve_528: false,
        });
        let picture = ExternalFileName::from_uuid_and_display(uuid, display).filename();
        let size = String::from("528");

        let avatar =
            retrieve_avatar_from_store(&settings, &loader, &picture, Some(&size), None).wait()?;

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
        let loader = Arc::new(DummyLoader { retrieve_528: true });
        let picture = ExternalFileName::from_uuid_and_display(uuid, display).filename();
        let size = String::from("528");

        let avatar =
            retrieve_avatar_from_store(&settings, &loader, &picture, Some(&size), None).wait()?;

        assert_eq!(avatar.len(), 528);
        Ok(())
    }
}
