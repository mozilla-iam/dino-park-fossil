#[cfg(feature = "local-fs")]
pub mod filesystem;

#[cfg(not(feature = "local-fs"))]
pub mod s3;

use failure::Error;
use futures::future::BoxFuture;

pub trait Loader: Sync + Send + Sized {
    fn load(&self, name: &str, prefix: &str, bucket: &str) -> BoxFuture<Result<Vec<u8>, Error>>;
}
