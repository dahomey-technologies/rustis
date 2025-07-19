use crate::Error;

/// Redis version returned by the hello command
pub struct Version {
    pub major: u8,
    #[allow(dead_code)]
    pub minor: u8,
    #[allow(dead_code)]
    pub revision: u8,
}

impl TryFrom<&str> for Version {
    type Error = Error;

    fn try_from(value: &str) -> std::result::Result<Self, Self::Error> {
        let mut split = value.split('.');

        let (Some(major), Some(minor), Some(revision), None) =
            (split.next(), split.next(), split.next(), split.next())
        else {
            return Err(Error::Client(
                "Cannot parse Redis server version".to_owned(),
            ));
        };

        let (Some(major), Some(minor), Some(revision)) = (
            atoi::atoi(major.as_bytes()),
            atoi::atoi(minor.as_bytes()),
            atoi::atoi(revision.as_bytes()),
        ) else {
            return Err(Error::Client(
                "Cannot parse Redis server version".to_owned(),
            ));
        };

        Ok(Version {
            major,
            minor,
            revision,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::Version;

    #[test]
    fn version() {
        let version: Version = "7.0.0".try_into().unwrap();
        assert_eq!((7, 0, 0), (version.major, version.minor, version.revision));
    }
}
