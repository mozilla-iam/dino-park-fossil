use chrono::Utc;
use cis_profile::schema::Display;
use failure::Error;
use sha2::Digest;
use std::convert::TryInto;
use std::fmt;

static FILE_ENDING: &str = "png";

#[derive(Debug, Fail)]
pub enum NameError {
    #[fail(display = "invalid utf8 in picture name")]
    InvalidUtf8,
    #[fail(display = "invalid picture name")]
    InvalidName,
}

pub fn uuid_hash(uuid: &str) -> String {
    format!("{:x}", sha2::Sha256::digest(uuid.as_bytes()))
}

pub struct InternalFileName {
    pub uuid_hash: String,
    pub display: Display,
}

impl fmt::Display for InternalFileName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}_{}.{}",
            &self.uuid_hash,
            &self.display.as_str(),
            FILE_ENDING
        )
    }
}

impl InternalFileName {
    pub fn from_uuid_and_display(uuid: &str, display: &Display) -> Self {
        InternalFileName {
            uuid_hash: uuid_hash(uuid),
            display: display.to_owned(),
        }
    }
}

pub struct ExternalFileName {
    pub internal: InternalFileName,
    pub ts: i64,
}

/// Represents an extenal filename.
impl ExternalFileName {
    /// Create a new `ExternalFileName` instance with the current timestamp.
    pub fn from_uuid_and_display(uuid: &str, display: &Display) -> Self {
        ExternalFileName {
            internal: InternalFileName::from_uuid_and_display(uuid, display),
            ts: Utc::now().timestamp(),
        }
    }
    pub fn from_uri(uri: &str) -> Result<Self, Error> {
        let encoded = match (uri.rfind('/'), uri.rfind('.')) {
            (Some(start), Some(end)) => &uri[start + 1..end],
            (Some(start), None) => &uri[start + 1..],
            (None, Some(end)) => &uri[..end],
            _ => uri,
        };
        Self::from_encoded(encoded)
    }
    pub fn from_encoded(encoded: &str) -> Result<Self, Error> {
        let decoded = base64::decode_config(encoded, base64::URL_SAFE_NO_PAD)?;
        let s = String::from_utf8(decoded).map_err(|_| NameError::InvalidUtf8)?;
        let mut parts = s.split('#').take(3).map(String::from);
        let uuid_hash = parts.next().ok_or_else(|| NameError::InvalidName)?;
        let display = parts
            .next()
            .ok_or_else(|| NameError::InvalidName)?
            .as_str()
            .try_into()?;
        let ts = parts.next().ok_or_else(|| NameError::InvalidName)?;
        Ok(ExternalFileName {
            internal: InternalFileName { uuid_hash, display },
            ts: ts.parse()?,
        })
    }

    pub fn encode(&self) -> String {
        base64::encode_config(
            &format!(
                "{}#{}#{}",
                &self.internal.uuid_hash,
                &self.internal.display.as_str(),
                self.ts
            ),
            base64::URL_SAFE_NO_PAD,
        )
    }
    pub fn filename(&self) -> String {
        format!("{}.{}", self.encode(), FILE_ENDING)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_name_uuid_conversion() -> Result<(), Error> {
        let uuid = "9e697947-2990-4182-b080-533c16af4799";
        let display = &Display::Staff;
        let name = ExternalFileName::from_uuid_and_display(uuid, display).filename();
        println!("{}", name);
        let external_file_name = ExternalFileName::from_uri(&name)?;
        assert_eq!(
            external_file_name.internal.uuid_hash,
            format!("{:x}", sha2::Sha256::digest(uuid.as_bytes()))
        );
        assert_eq!(&external_file_name.internal.display, display);
        Ok(())
    }
}
