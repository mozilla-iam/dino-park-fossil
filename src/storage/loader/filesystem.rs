use super::Loader;
use async_std::fs;
use failure::Error;
use futures::future::BoxFuture;
use log::info;
use std::path::PathBuf;
use std::sync::Arc;

pub struct FilesystemLoader {
    pub path: Arc<PathBuf>,
}

impl Loader for FilesystemLoader {
    fn load(&self, name: &str, prefix: &str, bucket: &str) -> BoxFuture<Result<Vec<u8>, Error>> {
        info!("reading file in bucket '{}'", bucket);

        let path = self
            .path
            .join(bucket.to_string())
            .join(format!("{}-{}", prefix, name));

        Box::pin(async move { Ok(fs::read(path).await?) })
    }
}
