use failure::Error;
use futures::Future;
use futures::Stream;
use log::info;
use rusoto_s3::GetObjectRequest;
use rusoto_s3::S3;

#[derive(Debug, Fail)]
pub enum S3Error {
    #[fail(display = "empty body received")]
    NoBody,
}

pub trait Loader {
    fn load(
        &self,
        name: &str,
        prefix: &str,
        bucket: &str,
    ) -> Box<dyn Future<Item = Vec<u8>, Error = Error>>;
}

#[derive(Clone)]
pub struct S3Loader<S: S3> {
    pub s3_client: S,
}

impl<S: S3> Loader for S3Loader<S> {
    fn load(
        &self,
        name: &str,
        prefix: &str,
        bucket: &str,
    ) -> Box<dyn Future<Item = Vec<u8>, Error = Error>> {
        let download = GetObjectRequest {
            bucket: bucket.to_owned(),
            key: format!("{}/{}", prefix, name),
            ..Default::default()
        };
        let name = name.to_owned();
        let bucket = bucket.to_owned();
        Box::new(
            self.s3_client
                .get_object(download)
                .map(move |res| {
                    info!(
                        "downloaded {} from {} with version_id: {}",
                        name,
                        bucket,
                        res.version_id
                            .as_ref()
                            .map(|x| x.as_str())
                            .unwrap_or_else(|| "-"),
                    );
                    res
                })
                .map_err(Error::from)
                .and_then(|res| res.body.ok_or_else(|| S3Error::NoBody.into()))
                .map_err(Error::from)
                .and_then(|body| body.concat2().map_err(Error::from))
                .map(|body| body.to_vec())
                .map_err(Into::into),
        )
    }
}
