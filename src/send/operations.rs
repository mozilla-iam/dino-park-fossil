use crate::send::resize::Avatars;
use crate::storage::loader::Loader;
use crate::storage::saver::Saver;
use failure::Error;

const LARGE: &str = "264";
const MEDIUM: &str = "100";
const SMALL: &str = "40";

pub fn delete(name: &str, bucket: &str, saver: &impl Saver) -> Result<(), Error> {
    saver.delete(name, LARGE, bucket)?;
    saver.delete(name, MEDIUM, bucket)?;
    saver.delete(name, SMALL, bucket)
}

pub fn save(avatars: Avatars, name: &str, bucket: &str, saver: &impl Saver) -> Result<(), Error> {
    let Avatars { x264, x100, x40 } = avatars;
    saver.save(name, LARGE, bucket, x264)?;
    saver.save(name, MEDIUM, bucket, x100)?;
    saver.save(name, SMALL, bucket, x40)
}

pub fn rename(
    old_name: &str,
    new_name: &str,
    bucket: &str,
    saver: &impl Saver,
    loader: &impl Loader,
) -> Result<(), Error> {
    if old_name == new_name {
        return Ok(());
    }
    rename_one(old_name, new_name, LARGE, bucket, saver, loader)?;
    rename_one(old_name, new_name, MEDIUM, bucket, saver, loader)?;
    rename_one(old_name, new_name, SMALL, bucket, saver, loader)
}

fn rename_one(
    old_name: &str,
    new_name: &str,
    size: &str,
    bucket: &str,
    saver: &impl Saver,
    loader: &impl Loader,
) -> Result<(), Error> {
    let buf = loader.load(old_name, size, bucket)?;
    saver.save(new_name, size, bucket, buf)?;
    saver.delete(old_name, size, bucket)
}
