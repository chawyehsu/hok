use std::{
    fmt::{self, Formatter},
    str::FromStr,
};

use once_cell::sync::Lazy;
use regex::{Regex, RegexBuilder};
use serde::{
    de::{self, SeqAccess, Visitor},
    Deserialize, Deserializer,
};

#[derive(Debug)]
/// `HashParseError` represents a error occuring when trying to create a
/// `HashString` from a string that does not satisfy the format requirement.
///
/// Refer to [`HashString`] for the format requirement.
pub struct HashParseError(String);

#[derive(Clone, Debug, Deserialize, Serialize)]
/// `HashString` represents a valid hash string used in the Scoop app manifest.
/// Currently, it could be one of the following formats:
///
/// - **md5**: `^md5:[a-fA-F0-9]{32}$`
/// - **sha1**: `^sha1:[a-fA-F0-9]{40}$`
/// - **sha256**: `^(sha256:)?[a-fA-F0-9]{64}$`
/// - **sha512**: `^sha512:[a-fA-F0-9]{128}$`
pub struct HashString {
    raw: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Hash {
    inner: Vec<HashString>,
}

impl fmt::Display for HashParseError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(self.0.as_str())
    }
}

impl std::error::Error for HashParseError {}

impl FromStr for HashString {
    type Err = HashParseError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match Self::validate(s) {
            false => {
                let msg = format!("{} is not a valid hash string", s);
                Err(HashParseError(msg))
            }
            true => Ok(Self {
                raw: String::from(s),
            }),
        }
    }
}

impl HashString {
    fn validate(s: &str) -> bool {
        static REGEX_HASH: Lazy<Regex> = Lazy::new(|| {
            RegexBuilder::new(r"^md5:[a-fA-F0-9]{32}|sha1:[a-fA-F0-9]{40}|(sha256:)?[a-fA-F0-9]{64}|sha512:[a-fA-F0-9]{128}$")
                .build()
                .unwrap()
        });
        REGEX_HASH.is_match(s)
    }
}

pub fn deserialize_option_hash<'de, D>(deserializer: D) -> Result<Option<Hash>, D::Error>
where
    D: Deserializer<'de>,
{
    struct OptionalHashVisitor;
    impl<'de> Visitor<'de> for OptionalHashVisitor {
        type Value = Option<Hash>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("null or string or list of strings")
        }

        fn visit_none<E>(self) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(None)
        }

        fn visit_some<D>(self, d: D) -> Result<Self::Value, D::Error>
        where
            D: de::Deserializer<'de>,
        {
            let inner = d.deserialize_any(HashStringOrArrayVisitor)?;
            Ok(Some(Hash { inner }))
        }
    }

    struct HashStringOrArrayVisitor;
    impl<'de> Visitor<'de> for HashStringOrArrayVisitor {
        type Value = Vec<HashString>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("hash string or list of hash strings")
        }

        fn visit_str<E>(self, s: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            HashString::from_str(s)
                .map(|hs| vec![hs])
                .map_err(E::custom)
        }

        fn visit_seq<S>(self, mut seq: S) -> Result<Self::Value, S::Error>
        where
            S: SeqAccess<'de>,
        {
            let mut v: Vec<HashString> = Vec::new();
            while let Some(item) = seq.next_element()? {
                match HashString::from_str(item).map_err(de::Error::custom) {
                    Ok(hs) => v.push(hs),
                    Err(e) => return Err(e),
                }
            }

            Ok(v)
        }
    }

    deserializer.deserialize_option(OptionalHashVisitor)
}
