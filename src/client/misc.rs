use std::fmt;
use std::str::FromStr;

use ::serde::{de, Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AuthorizationCode(());

struct AuthorizationCodeVisitor;

impl AuthorizationCode {
    pub const STR: &'static str = "authorization_code";

    pub fn new() -> Self {
        Self(())
    }
}

impl fmt::Display for AuthorizationCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(Self::STR)
    }
}

impl FromStr for AuthorizationCode {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s == Self::STR {
            return Ok(Self::new());
        }
        Err("not authorization_code")
    }
}

impl Serialize for AuthorizationCode {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(Self::STR)
    }
}

impl<'de> Deserialize<'de> for AuthorizationCode {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        deserializer.deserialize_str(AuthorizationCodeVisitor)
    }
}

impl<'de> de::Visitor<'de> for AuthorizationCodeVisitor {
    type Value = AuthorizationCode;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str(r#"a str "authorization_code""#)
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        v.parse().map_err(E::custom)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Bearer(());

struct BearerVisitor;

impl Bearer {
    pub const STR: &'static str = "Bearer";

    fn new() -> Self {
        Self(())
    }
}

impl fmt::Display for Bearer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(Self::STR)
    }
}

impl FromStr for Bearer {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s == Self::STR {
            Ok(Self::new())
        } else {
            Err("not Bearer")
        }
    }
}

impl Serialize for Bearer {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(Self::STR)
    }
}

impl<'de> Deserialize<'de> for Bearer {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        deserializer.deserialize_str(BearerVisitor)
    }
}

impl<'de> de::Visitor<'de> for BearerVisitor {
    type Value = Bearer;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str(r#"a str "Bearer""#)
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        v.parse().map_err(E::custom)
    }
}
