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

        let path = self.path.join(bucket).join(format!("{}-{}", prefix, name));

        Box::pin(async move { Ok(fs::read(path).await?) })
    }
}

#[cfg(test)]
mod tests {
    use super::FilesystemLoader;
    use crate::storage::loader::Loader;
    use std::io;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_read_access() -> io::Result<()> {
        const BUCKET: &str = "test_read_access";

        let loader = FilesystemLoader {
            path: Arc::new(std::env::temp_dir()),
        };

        // create test directory
        match std::fs::create_dir(std::env::temp_dir().join(BUCKET)) {
            Ok(()) => (),
            Err(err) => match err.kind() {
                // ignore the error
                std::io::ErrorKind::AlreadyExists => (),
                _ => panic!("Error occured while setting up test: {:?}", err),
            },
        };

        // write file to read
        std::fs::write(
            std::env::temp_dir().join(BUCKET).join("1337-test.txt"),
            b"hello world",
        )?;

        let result = loader.load("test.txt", "1337", BUCKET).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), b"hello world".to_vec());

        std::fs::remove_file(std::env::temp_dir().join(BUCKET).join("1337-test.txt"))?;

        Ok(())
    }

    #[tokio::test]
    async fn read_non_existant_file_returns_err() {
        const BUCKET: &str = "non_existant_bucket";
        let loader = FilesystemLoader {
            path: Arc::new(std::env::temp_dir()),
        };

        let result = loader.load("dontusemyname.txt", "1337-pro", BUCKET).await;

        assert!(result.is_err());
        assert_eq!(
            result
                .unwrap_err()
                .downcast_ref::<std::io::Error>()
                .unwrap()
                .kind(),
            std::io::ErrorKind::NotFound
        );
    }
}
