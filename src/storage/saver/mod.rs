#[cfg(feature = "local-fs")]
pub mod filesystem;

#[cfg(not(feature = "local-fs"))]
pub mod s3;

use failure::Error;
use futures::future::BoxFuture;

pub trait Saver: Sync + Send + Sized {
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
