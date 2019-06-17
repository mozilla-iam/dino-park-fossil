use failure::Error;
use futures::Future;
use rusoto_s3::DeleteObjectRequest;
use rusoto_s3::PutObjectRequest;
use rusoto_s3::S3;

pub trait Saver {
    fn save(
        &self,
        name: &str,
        prefix: &str,
        bucket: &str,
        buf: Vec<u8>,
    ) -> Box<Future<Item = (), Error = Error>>;
    fn delete(
        &self,
        name: &str,
        prefix: &str,
        bucket: &str,
    ) -> Box<Future<Item = (), Error = Error>>;
}

#[derive(Clone)]
pub struct S3Saver<S: S3> {
    pub s3_client: S,
}

impl<S: S3> Saver for S3Saver<S> {
    fn save(
        &self,
        name: &str,
        prefix: &str,
        bucket: &str,
        buf: Vec<u8>,
    ) -> Box<Future<Item = (), Error = Error>> {
        let put = PutObjectRequest {
            bucket: bucket.to_owned(),
            key: format!("{}/{}", prefix, name),
            body: Some(buf.into()),
            ..Default::default()
        };
        let name = name.to_owned();
        let bucket = bucket.to_owned();
        Box::new(
            self.s3_client
                .put_object(put)
                .map_err(Error::from)
                .map(move |res| {
                    info!(
                        "uploaded {} to {} with version_id: {}",
                        name,
                        bucket,
                        res.version_id
                            .as_ref()
                            .map(|x| x.as_str())
                            .unwrap_or_else(|| "-"),
                    );
                }),
        )
    }
    fn delete(
        &self,
        name: &str,
        prefix: &str,
        bucket: &str,
    ) -> Box<Future<Item = (), Error = Error>> {
        let delete = DeleteObjectRequest {
            bucket: bucket.to_owned(),
            key: format!("{}/{}", prefix, name),
            ..Default::default()
        };
        let name = name.to_owned();
        let bucket = bucket.to_owned();
        Box::new(
            self.s3_client
                .delete_object(delete)
                .map_err(Error::from)
                .map(move |res| {
                    info!(
                        "deleted {} from {} with version_id: {}",
                        name,
                        bucket,
                        res.version_id
                            .as_ref()
                            .map(|x| x.as_str())
                            .unwrap_or_else(|| "-"),
                    );
                }),
        )
    }
}
