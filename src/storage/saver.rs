use chrono::Duration;
use chrono::Utc;
use failure::Error;
use futures::future::BoxFuture;
use log::debug;
use log::info;
use rusoto_s3::Delete;
use rusoto_s3::DeleteObjectRequest;
use rusoto_s3::DeleteObjectsRequest;
use rusoto_s3::ObjectIdentifier;
use rusoto_s3::PutObjectRequest;
use rusoto_s3::S3;
use std::ops::Add;
use uuid::Uuid;

pub trait Saver {
    fn save(
        &self,
        name: &str,
        prefix: &str,
        bucket: &str,
        buf: Vec<u8>,
    ) -> BoxFuture<Result<(), Error>>;
    fn delete(&self, name: &str, prefix: &str, bucket: &str) -> BoxFuture<Result<(), Error>>;
    fn delete_many(
        &self,
        names: &[String],
        prefix: &str,
        bucket: &str,
    ) -> BoxFuture<Result<(), Error>>;
    fn save_tmp(&self, bucket: &str, buf: Vec<u8>) -> BoxFuture<Result<String, Error>>;
}

#[derive(Clone)]
pub struct S3Saver<S: S3 + Send + Sync> {
    pub s3_client: S,
}

impl<S: S3 + Send + Sync> Saver for S3Saver<S> {
    fn save(
        &self,
        name: &str,
        prefix: &str,
        bucket: &str,
        buf: Vec<u8>,
    ) -> BoxFuture<Result<(), Error>> {
        let put = PutObjectRequest {
            bucket: bucket.to_owned(),
            key: format!("{}/{}", prefix, name),
            body: Some(buf.into()),
            ..Default::default()
        };
        let name = name.to_owned();
        let bucket = bucket.to_owned();
        Box::pin(async move {
            let res = self.s3_client.put_object(put).await?;
            info!(
                "uploaded {} to {} with version_id: {}",
                name,
                bucket,
                res.version_id.as_deref().unwrap_or_else(|| "-"),
            );
            Ok(())
        })
    }
    fn delete(&self, name: &str, prefix: &str, bucket: &str) -> BoxFuture<Result<(), Error>> {
        let delete = DeleteObjectRequest {
            bucket: bucket.to_owned(),
            key: format!("{}/{}", prefix, name),
            ..Default::default()
        };
        let name = name.to_owned();
        let bucket = bucket.to_owned();
        Box::pin(async move {
            let res = self.s3_client.delete_object(delete).await?;
            info!(
                "deleted {} from {} with version_id: {}",
                name,
                bucket,
                res.version_id.as_deref().unwrap_or_else(|| "-"),
            );
            Ok(())
        })
    }
    fn delete_many(
        &self,
        names: &[String],
        prefix: &str,
        bucket: &str,
    ) -> BoxFuture<Result<(), Error>> {
        let delete = Delete {
            objects: names
                .iter()
                .map(|name| ObjectIdentifier {
                    key: format!("{}/{}", prefix, name),
                    ..Default::default()
                })
                .collect(),
            quiet: None,
        };
        let delete_req = DeleteObjectsRequest {
            bucket: bucket.to_owned(),
            delete,
            ..Default::default()
        };
        let bucket = bucket.to_owned();
        Box::pin(async move {
            let res = self.s3_client.delete_objects(delete_req).await?;
            let deleted = res.deleted.unwrap_or_default();
            let errors = res.errors.unwrap_or_default();
            debug!(
                "deleted [{}] from {}. ([{}])",
                deleted
                    .into_iter()
                    .map(|d| d.key.unwrap_or_default())
                    .collect::<Vec<_>>()
                    .join(", "),
                bucket,
                errors
                    .into_iter()
                    .map(|e| e.key.unwrap_or_default())
                    .collect::<Vec<_>>()
                    .join(", "),
            );
            Ok(())
        })
    }
    fn save_tmp(&self, bucket: &str, buf: Vec<u8>) -> BoxFuture<Result<String, Error>> {
        let name = Uuid::new_v4().to_simple().to_string();
        let put = PutObjectRequest {
            bucket: bucket.to_owned(),
            key: format!("tmp/{}", &name),
            body: Some(buf.into()),
            expires: Some(Utc::now().add(Duration::hours(1)).to_rfc3339()),
            ..Default::default()
        };
        let bucket = bucket.to_owned();
        Box::pin(async move {
            let res = self.s3_client.put_object(put).await?;
            info!(
                "created tmp file {} in {} with version_id: {}",
                name,
                bucket,
                res.version_id.as_deref().unwrap_or_else(|| "-"),
            );
            Ok(name)
        })
    }
}
