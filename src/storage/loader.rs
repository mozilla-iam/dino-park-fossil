use failure::Error;
use futures::Future;
use futures::Stream;
use rusoto_s3::GetObjectRequest;
use rusoto_s3::S3;

#[derive(Debug, Fail)]
pub enum S3Error {
    #[fail(display = "empty body received")]
    NoBody,
}

pub trait Loader {
    fn load(&self, name: &str, prefix: &str, bucket: &str) -> Result<Vec<u8>, Error>;
}

#[derive(Clone)]
pub struct S3Loader<S: S3> {
    pub s3_client: S,
}

impl<S: S3> Loader for S3Loader<S> {
    fn load(&self, name: &str, prefix: &str, bucket: &str) -> Result<Vec<u8>, Error> {
        let download = GetObjectRequest {
            bucket: bucket.to_owned(),
            key: format!("{}/{}", prefix, name),
            ..Default::default()
        };
        let res = self.s3_client.get_object(download).sync()?;
        info!(
            "downloaded {} from {} with version_id: {}",
            name,
            bucket,
            res.version_id.unwrap_or_else(|| String::from("-")),
        );
        if let Some(body) = res.body {
            let buf: Vec<u8> = body.concat2().wait()?.to_vec();
            Ok(buf)
        } else {
            Err(S3Error::NoBody.into())
        }
    }
}
