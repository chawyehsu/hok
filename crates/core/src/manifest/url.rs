use serde::{
    de::{self, SeqAccess, Visitor},
    Deserialize, Deserializer,
};
use std::fmt;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Url {
    inner: Vec<String>,
}

pub struct UrlIterator<'a> {
    inner: std::slice::Iter<'a, String>,
}

impl<'a> Iterator for UrlIterator<'a> {
    type Item = &'a String;
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }
}

impl Url {
    pub fn iter<'a>(&'a self) -> UrlIterator<'a> {
        UrlIterator {
            inner: self.inner.iter(),
        }
    }
}

struct UrlVisitor;
impl<'de> Visitor<'de> for UrlVisitor {
    type Value = Url;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("url string or list of url strings")
    }

    fn visit_str<E>(self, s: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(Url {
            inner: vec![s.to_string()],
        })
    }

    fn visit_seq<S>(self, mut seq: S) -> Result<Self::Value, S::Error>
    where
        S: SeqAccess<'de>,
    {
        let mut v: Vec<String> = Vec::new();
        while let Some(item) = seq.next_element()? {
            v.push(item)
        }

        Ok(Url { inner: v })
    }
}

#[allow(unused)]
pub fn deserialize_url<'de, D>(deserializer: D) -> Result<Url, D::Error>
where
    D: Deserializer<'de>,
{
    deserializer.deserialize_any(UrlVisitor)
}

pub fn deserialize_option_url<'de, D>(deserializer: D) -> Result<Option<Url>, D::Error>
where
    D: Deserializer<'de>,
{
    struct OptionalUrlVisitor;
    impl<'de> Visitor<'de> for OptionalUrlVisitor {
        type Value = Option<Url>;

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
            Ok(Some(d.deserialize_any(UrlVisitor)?))
        }
    }

    deserializer.deserialize_option(OptionalUrlVisitor)
}
