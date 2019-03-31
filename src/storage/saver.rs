use failure::Error;
use rusoto_s3::DeleteObjectRequest;
use rusoto_s3::PutObjectRequest;
use rusoto_s3::S3;

pub trait Saver {
    fn save(&self, name: &str, prefix: &str, bucket: &str, buf: Vec<u8>) -> Result<(), Error>;
    fn delete(&self, name: &str, prefix: &str, bucket: &str) -> Result<(), Error>;
}

#[derive(Clone)]
pub struct S3Saver<S: S3> {
    pub s3_client: S,
}

impl<S: S3> Saver for S3Saver<S> {
    fn save(&self, name: &str, prefix: &str, bucket: &str, buf: Vec<u8>) -> Result<(), Error> {
        let put = PutObjectRequest {
            bucket: bucket.to_owned(),
            key: format!("{}/{}", prefix, name),
            body: Some(buf.into()),
            ..Default::default()
        };
        let res = self.s3_client.put_object(put).sync()?;
        info!(
            "uploaded {} to {} with version_id: {}",
            name,
            bucket,
            res.version_id.unwrap_or_else(|| String::from("-")),
        );
        Ok(())
    }
    fn delete(&self, name: &str, prefix: &str, bucket: &str) -> Result<(), Error> {
        let delete = DeleteObjectRequest {
            bucket: bucket.to_owned(),
            key: format!("{}/{}", prefix, name),
            ..Default::default()
        };
        let res = self.s3_client.delete_object(delete).sync()?;
        info!(
            "deleted {} from {} with version_id: {}",
            name,
            bucket,
            res.version_id.unwrap_or_else(|| String::from("-")),
        );
        Ok(())
    }
}
