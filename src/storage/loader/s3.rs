use super::Loader;
use bytes::BytesMut;
use failure::Error;
use futures::future::BoxFuture;
use futures::stream::TryStreamExt;
use log::info;
use rusoto_s3::GetObjectRequest;
use rusoto_s3::S3Client;
use rusoto_s3::S3;

#[derive(Debug, Fail)]
pub enum S3Error {
    #[fail(display = "empty body received")]
    NoBody,
}

#[derive(Clone)]
pub struct S3Loader {
    pub s3_client: S3Client,
}

impl Loader for S3Loader {
    fn load(&self, name: &str, prefix: &str, bucket: &str) -> BoxFuture<Result<Vec<u8>, Error>> {
        let download = GetObjectRequest {
            bucket: bucket.to_owned(),
            key: format!("{}/{}", prefix, name),
            ..Default::default()
        };
        let name = name.to_owned();
        let bucket = bucket.to_owned();
        Box::pin(async move {
            let res = self.s3_client.get_object(download).await?;
            info!(
                "downloaded {} from {} with version_id: {}",
                name,
                bucket,
                res.version_id.as_deref().unwrap_or_else(|| "-"),
            );
            let stream = res.body.ok_or_else(|| S3Error::NoBody)?;
            let body = stream
                .map_ok(|b| BytesMut::from(&b[..]))
                .try_concat()
                .await?;
            Ok(body.to_vec())
        })
    }
}
