use crate::send::resize::Avatars;
use crate::storage::loader::Loader;
use crate::storage::saver::Saver;
use failure::Error;
use future::FutureExt;
use futures::future;
use log::warn;
use std::sync::Arc;

const RAW: &str = "raw";
const XLARGE: &str = "528";
const LARGE: &str = "264";
const MEDIUM: &str = "100";
const SMALL: &str = "40";

pub async fn delete(name: &str, bucket: &str, saver: &Arc<impl Saver>) -> Result<(), Error> {
    future::try_join5(
        saver.delete(name, RAW, bucket).map(|r| match r {
            Ok(_) => Ok(()),
            Err(e) => {
                warn!("unable to delte RAW picture: {}", e);
                Ok::<_, Error>(())
            }
        }),
        saver.delete(name, XLARGE, bucket).map(|r| match r {
            Ok(_) => Ok(()),
            Err(e) => {
                warn!("unable to delte XLARGE picture: {}", e);
                Ok::<_, Error>(())
            }
        }),
        saver.delete(name, LARGE, bucket),
        saver.delete(name, MEDIUM, bucket),
        saver.delete(name, SMALL, bucket),
    )
    .await?;
    Ok(())
}

pub async fn save(
    avatars: Avatars,
    name: &str,
    bucket: &str,
    saver: &Arc<impl Saver>,
) -> Result<(), Error> {
    let Avatars {
        raw,
        x528,
        x264,
        x100,
        x40,
    } = avatars;
    future::try_join5(
        saver.save(name, RAW, bucket, raw),
        saver.save(name, XLARGE, bucket, x528),
        saver.save(name, LARGE, bucket, x264),
        saver.save(name, MEDIUM, bucket, x100),
        saver.save(name, SMALL, bucket, x40),
    )
    .await?;
    Ok(())
}

pub async fn rename(
    old_name: &str,
    new_name: &str,
    bucket: &str,
    saver: &Arc<impl Saver>,
    loader: &Arc<impl Loader>,
) -> Result<(), Error> {
    if old_name == new_name {
        return Ok(());
    }
    future::try_join5(
        rename_one(old_name, new_name, RAW, bucket, saver, loader).map(|r| match r {
            Ok(_) => Ok(()),
            Err(e) => {
                warn!("unable to rename RAW picture: {}", e);
                Ok::<_, Error>(())
            }
        }),
        rename_one(old_name, new_name, XLARGE, bucket, saver, loader).map(|r| match r {
            Ok(_) => Ok(()),
            Err(e) => {
                warn!("unable to rename XLARGE picture: {}", e);
                Ok::<_, Error>(())
            }
        }),
        rename_one(old_name, new_name, LARGE, bucket, saver, loader),
        rename_one(old_name, new_name, MEDIUM, bucket, saver, loader),
        rename_one(old_name, new_name, SMALL, bucket, saver, loader),
    )
    .await?;
    Ok(())
}

async fn rename_one(
    old_name: &str,
    new_name: &str,
    size: &str,
    bucket: &str,
    saver: &Arc<impl Saver>,
    loader: &Arc<impl Loader>,
) -> Result<(), Error> {
    let buf = loader.load(old_name, &size, &bucket).await?;
    saver.save(new_name, &size, &bucket, buf).await?;
    saver.delete(old_name, &size, &bucket).await?;
    Ok(())
}
