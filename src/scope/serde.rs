use std::fmt;

use ::serde::{de, Deserialize, Serialize};

use super::{DynSingleScope, SpaceDelimitedScope};

impl Serialize for DynSingleScope {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.0.as_str())
    }
}

impl<'de> Deserialize<'de> for DynSingleScope {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        deserializer.deserialize_str(DynSingleScopeVisitor)
    }
}

struct DynSingleScopeVisitor;

impl<'de> de::Visitor<'de> for DynSingleScopeVisitor {
    type Value = DynSingleScope;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a str representing a single scope")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        v.parse().map_err(E::custom)
    }
}

impl Serialize for SpaceDelimitedScope {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for SpaceDelimitedScope {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        deserializer.deserialize_str(SpaceDelimitedScopeVisitor)
    }
}

struct SpaceDelimitedScopeVisitor;

impl<'de> de::Visitor<'de> for SpaceDelimitedScopeVisitor {
    type Value = SpaceDelimitedScope;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a str of space-delimited scope")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        v.parse().map_err(E::custom)
    }
}

macro_rules! serde_for_scope {
    [ $( $i0:ident $(. $i:ident)* ),* ] => { ::paste::paste! { $(
        impl ::serde::Serialize for super::[< $i0:camel $( $i:camel )* >] {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: ::serde::Serializer,
            {
                serializer.serialize_str(Self::STR)
            }
        }

        impl<'de> ::serde::Deserialize<'de> for super::[< $i0:camel $( $i:camel )* >] {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: ::serde::de::Deserializer<'de>,
            {
                deserializer.deserialize_str([< $i0:camel $( $i:camel )* Visitor >])
            }
        }

        struct [< $i0:camel $( $i:camel )* Visitor >];

        impl<'de> ::serde::de::Visitor<'de> for [< $i0:camel $( $i:camel )* Visitor >] {
            type Value = super::[< $i0:camel $( $i:camel )* >];

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                write!(formatter, r#"a str "{}""#, super::[< $i0:camel $( $i:camel )* >]::STR)
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: ::serde::de::Error,
            {
                v.parse().map_err(E::custom)
            }
        }

    )* } };
}

super::apply_all_scope! {serde_for_scope}
