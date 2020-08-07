use super::Saver;
use async_std::fs;
use async_std::prelude::*;
use failure::Error;
use futures::future::BoxFuture;
use futures::FutureExt;
use futures::TryFutureExt;
use log::info;
use log::warn;
use std::env;
use std::path::PathBuf;
use std::sync::Arc;
use uuid::Uuid;

pub struct FilesystemSaver {
    pub path: Arc<PathBuf>,
}

impl Saver for FilesystemSaver {
    fn save(
        &self,
        name: &str,
        prefix: &str,
        bucket: &str,
        buf: Vec<u8>,
    ) -> BoxFuture<Result<(), Error>> {
        let path = self.path.clone().join(bucket.to_owned());
        info!("saving permanent file in {}", path.display());

        let file_name = format!("{}-{}", prefix, name);

        Box::pin(async move {
            match fs::create_dir_all(path.clone()).await {
                Ok(()) => (),
                // ignore error, as an error is also thrown when the directory already exists
                // if the error is fatal, the write operation will also fail
                Err(err) => warn!(
                    "creation of folder to save file was not successful: {}",
                    err
                ),
            };

            let mut file = fs::OpenOptions::new()
                .create(true)
                .write(true)
                .open(path.join(file_name))
                .await?;

            file.write(&buf).await?;

            Ok(())
        })
    }

    fn delete(&self, name: &str, prefix: &str, bucket: &str) -> BoxFuture<Result<(), Error>> {
        let path = self
            .path
            .join(bucket.to_string())
            .join(format!("{}-{}", prefix, name));

        let name = name.to_owned();
        let bucket = bucket.to_owned();

        Box::pin(async move {
            let result = match fs::remove_file(path).await {
                Ok(()) => Ok(()),
                Err(err) => match err.kind() {
                    std::io::ErrorKind::NotFound => Ok(()),
                    _ => return Err(err.into()),
                },
            };
            info!("deleted {} from {}", name, bucket);

            result
        })
    }

    fn delete_many(
        &self,
        names: &[String],
        prefix: &str,
        bucket: &str,
    ) -> BoxFuture<Result<(), Error>> {
        let prefix = prefix.to_owned();
        let bucket = bucket.to_owned();

        let names = names
            .iter()
            .map(|name| self.delete(&name, &prefix, &bucket));

        futures::future::try_join_all(names).map_ok(|_| ()).boxed()
    }

    fn save_tmp(&self, bucket: &str, buf: Vec<u8>) -> BoxFuture<Result<String, Error>> {
        info!("saving temporary file in bucket '{}'", bucket);

        let temp_dir = env::temp_dir();

        let file_uuid = Uuid::new_v4().to_simple().to_string();
        let file_name = format!("{}-{}", bucket, file_uuid);

        Box::pin(async move {
            let mut file = fs::OpenOptions::new()
                .create(true)
                .write(true)
                .open(temp_dir.join(file_name))
                .await?;

            file.write(&buf).await?;

            Ok(file_uuid)
        })
    }
}
