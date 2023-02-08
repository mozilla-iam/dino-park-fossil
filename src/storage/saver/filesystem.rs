use super::Saver;
use async_std::fs;
use async_std::prelude::*;
use failure::Error;
use futures::future::BoxFuture;
use futures::FutureExt;
use futures::TryFutureExt;
use log::error;
use log::info;
use log::warn;
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
        let path = self.path.clone().join(bucket);
        info!("saving permanent file in {}", path.display());

        let file_name = format!("{prefix}-{name}");

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
        let path = self.path.join(bucket).join(format!("{prefix}-{name}"));

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

        let names = names.iter().map(|name| self.delete(name, &prefix, &bucket));

        futures::future::try_join_all(names).map_ok(|_| ()).boxed()
    }

    fn save_tmp(&self, bucket: &str, buf: Vec<u8>) -> BoxFuture<Result<String, Error>> {
        info!("saving temporary file in bucket '{}'", bucket);

        let path = self.path.join(bucket);

        let file_uuid = Uuid::new_v4().to_simple().to_string();
        let file_name = format!("tmp-{file_uuid}");

        Box::pin(async move {
            match fs::create_dir_all(path.clone()).await {
                Ok(()) => (),
                // ignore error, as an error is also thrown when the directory already exists
                // if the error is fatal, the write operation will also fail
                Err(err) => error!(
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

            Ok(file_uuid)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::FilesystemSaver;
    use crate::storage::saver::Saver;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_simple_tmp_write() {
        const BUCKET: &str = "simple_tmp_write";

        let saver = FilesystemSaver {
            path: Arc::new(std::env::temp_dir()),
        };

        // write file to read
        let result = saver.save_tmp(BUCKET, b"hello world".to_vec()).await;

        assert!(result.is_ok());

        // make sure file holds the respective content
        assert_eq!(
            std::fs::read(
                std::env::temp_dir()
                    .join(BUCKET)
                    .join(format!("tmp-{}", result.unwrap()))
            )
            .unwrap(),
            b"hello world"
        );
    }

    #[tokio::test]
    async fn test_delete() {
        const BUCKET: &str = "delete_bucket";

        let saver = FilesystemSaver {
            path: Arc::new(std::env::temp_dir()),
        };

        // create bucket directory
        match std::fs::create_dir(std::env::temp_dir().join(BUCKET)) {
            Ok(()) => (),
            Err(err) => match err.kind() {
                // ignore the error
                std::io::ErrorKind::AlreadyExists => (),
                _ => panic!("Error occured while setting up test: {:?}", err),
            },
        };

        // write file to delete
        std::fs::write(
            std::env::temp_dir().join(BUCKET).join("pre-hello.txt"),
            b"hello world",
        )
        .unwrap();

        let delete_result = saver.delete("hello.txt", "pre", BUCKET).await;

        eprintln!("{delete_result:?}");
        // make sure that the file was successfully deleted
        assert!(delete_result.is_ok());

        let metadata_result =
            std::fs::metadata(std::env::temp_dir().join(BUCKET).join("pre-hello.txt"));

        // make sure file does not exist anymore
        assert!(metadata_result.is_err());
        assert_eq!(
            metadata_result.unwrap_err().kind(),
            std::io::ErrorKind::NotFound
        );
    }

    #[tokio::test]
    async fn test_delete_many() {
        const PREFIX: &str = "pre";
        const BUCKET: &str = "delete_many_bucket";

        let test_files = vec![
            (String::from("hello.txt"), b"world"),
            (String::from("world.txt"), b"hello"),
        ];

        let saver = FilesystemSaver {
            path: Arc::new(std::env::temp_dir()),
        };

        // create bucket directory
        match std::fs::create_dir(std::env::temp_dir().join(BUCKET)) {
            Ok(()) => (),
            Err(err) => match err.kind() {
                // ignore the error
                std::io::ErrorKind::AlreadyExists => (),
                _ => panic!("Error occured while setting up test: {:?}", err),
            },
        };

        // write files to delete
        for (name, content) in test_files.clone() {
            std::fs::write(
                std::env::temp_dir()
                    .join(BUCKET)
                    .join(format!("{PREFIX}-{name}")),
                content,
            )
            .unwrap();
        }

        let delete_result = saver
            .delete_many(
                &test_files
                    .clone()
                    .iter()
                    .map(|(name, _content)| name.to_owned())
                    .collect::<Vec<String>>()[..],
                PREFIX,
                BUCKET,
            )
            .await;

        eprintln!("{delete_result:?}");
        assert!(delete_result.is_ok());

        // make sure all the files were deleted
        for (name, _content) in test_files.clone() {
            let metadata_result = std::fs::metadata(
                std::env::temp_dir()
                    .join(BUCKET)
                    .join(format!("{PREFIX}-{name}")),
            );

            assert!(metadata_result.is_err());
            assert_eq!(
                metadata_result.unwrap_err().kind(),
                std::io::ErrorKind::NotFound
            );
        }
    }
}
