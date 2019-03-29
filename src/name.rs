use base64;
use chrono::Utc;
use failure::Error;

#[derive(Debug, Fail)]
pub enum NameError {
    #[fail(display = "invalid utf8 in picture name")]
    InvalidUtf8,
    #[fail(display = "invalid picture name")]
    InvalidName,
}

pub fn name_from_uuid(uuid: &str) -> String {
    base64::encode_config(
        &format!("{}#{}", uuid, Utc::now().timestamp()),
        base64::URL_SAFE_NO_PAD,
    )
}

pub fn uuid_from_name(name: &str) -> Result<String, Error> {
    let decoded = base64::decode_config(name, base64::URL_SAFE_NO_PAD)?;
    String::from_utf8(decoded)
        .map_err(|_| NameError::InvalidUtf8.into())
        .and_then(|s| {
            s.split('#')
                .next()
                .map(String::from)
                .ok_or_else(|| NameError::InvalidName.into())
        })
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_name_uuid_conversion() -> Result<(), Error> {
        let uuid = "9e697947-2990-4182-b080-533c16af4799";
        let name = name_from_uuid(uuid);
        let restored_uuid = uuid_from_name(&name)?;
        assert_eq!(restored_uuid, uuid);
        Ok(())
    }
}
