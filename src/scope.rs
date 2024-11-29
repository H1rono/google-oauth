use std::any::Any;
use std::collections::HashSet;
use std::fmt;
use std::hash::{Hash, Hasher};

mod private {
    pub trait Sealed {}
}

macro_rules! box_scope {
    ($e:expr) => {
        BoxScope(Box::new($e))
    };
}

pub trait SingleScope: private::Sealed + Send + Sync + 'static {
    fn as_any(&self) -> &dyn Any;

    fn as_dyn(&self) -> DynSingleScope;

    fn as_str(&self) -> &'static str;

    fn equals(&self, other: &dyn SingleScope) -> bool;

    fn hash_value(&self) -> u64;
}

pub trait Scope: private::Sealed + Send + Sync + 'static {
    fn scope(&self) -> HashSet<DynSingleScope>;

    fn scope_str(&self) -> HashSet<&'static str>;

    fn grants(&self, other: &dyn SingleScope) -> bool {
        let other = other.as_dyn();
        self.scope().contains(&other)
    }

    fn boxed_clone(&self) -> BoxScope;

    fn with<S: Scope>(self, scope: S) -> With<Self, S>
    where
        Self: Sized,
    {
        With(self, scope)
    }

    fn into_boxed(self) -> BoxScope
    where
        Self: Sized,
    {
        box_scope!(self)
    }
}

#[derive(Clone, Copy)]
pub struct DynSingleScope(&'static dyn SingleScope);

impl PartialEq for DynSingleScope {
    fn eq(&self, other: &Self) -> bool {
        self.0.equals(other.0)
    }
}

impl Eq for DynSingleScope {}

impl Hash for DynSingleScope {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let v = self.0.hash_value();
        v.hash(state);
    }
}

impl From<&'static dyn SingleScope> for DynSingleScope {
    fn from(value: &'static dyn SingleScope) -> Self {
        Self(value)
    }
}

impl From<DynSingleScope> for &'static dyn SingleScope {
    fn from(value: DynSingleScope) -> Self {
        value.0
    }
}

impl private::Sealed for DynSingleScope {}

impl SingleScope for DynSingleScope {
    fn as_any(&self) -> &dyn Any {
        self.0.as_any()
    }

    fn as_dyn(&self) -> DynSingleScope {
        *self
    }

    fn as_str(&self) -> &'static str {
        self.0.as_str()
    }

    fn equals(&self, other: &dyn SingleScope) -> bool {
        self.0.equals(other)
    }

    fn hash_value(&self) -> u64 {
        self.0.hash_value()
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct NoScope;

impl private::Sealed for NoScope {}

impl Scope for NoScope {
    #[inline]
    fn scope(&self) -> HashSet<DynSingleScope> {
        HashSet::new()
    }

    #[inline]
    fn scope_str(&self) -> HashSet<&'static str> {
        HashSet::new()
    }

    #[inline]
    fn grants(&self, _other: &dyn SingleScope) -> bool {
        false
    }

    #[inline]
    fn boxed_clone(&self) -> BoxScope {
        box_scope!(*self)
    }
}

pub struct BoxScope(Box<dyn Scope>);

impl private::Sealed for BoxScope {}

impl Scope for BoxScope {
    fn scope(&self) -> HashSet<DynSingleScope> {
        self.0.scope()
    }

    fn scope_str(&self) -> HashSet<&'static str> {
        self.0.scope_str()
    }

    fn boxed_clone(&self) -> BoxScope {
        self.0.boxed_clone()
    }
}

impl Clone for BoxScope {
    fn clone(&self) -> Self {
        self.boxed_clone()
    }
}

impl fmt::Debug for BoxScope {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("BoxScope").field(&self.scope_str()).finish()
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct With<A, B>(A, B);

impl<A: private::Sealed, B: private::Sealed> private::Sealed for With<A, B> {}

impl<A, B> Scope for With<A, B>
where
    A: Scope,
    B: Scope,
    Self: Clone,
{
    fn scope(&self) -> HashSet<DynSingleScope> {
        let Self(a, b) = self;
        let mut scope_a = a.scope();
        let scope_b = b.scope();
        scope_a.extend(scope_b);
        scope_a
    }

    fn scope_str(&self) -> HashSet<&'static str> {
        let Self(a, b) = self;
        let mut scope_a = a.scope_str();
        let scope_b = b.scope_str();
        scope_a.extend(scope_b);
        scope_a
    }

    fn boxed_clone(&self) -> BoxScope {
        box_scope!(self.clone())
    }
}

macro_rules! scope {
    ( $(
        $( #[$m:meta] )*
        $i0:ident $(. $i:ident)* ;
    )+ ) => { ::paste::paste! { $(
        $( #[$m:meta] )*
        #[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
        pub struct [< $i0:camel $( $i:camel )* >];

        impl [< $i0:camel $( $i:camel )* >] {
            pub const STR: &'static str = concat!(
                "https://www.googleapis.com/auth/",
                stringify!($i0)
                $(, ".", stringify!($i))*
            );

            pub fn new() -> Self {
                Self
            }
        }

        impl fmt::Display for [< $i0:camel $( $i:camel )* >] {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(Self::STR)
            }
        }

        impl ::std::str::FromStr for [< $i0:camel $( $i:camel )* >] {
            type Err = String;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                if s == Self::STR {
                    Ok(Self)
                } else {
                    Err(format!("expected {}", Self::STR))
                }
            }
        }

        impl private::Sealed for [< $i0:camel $( $i:camel )* >] {}

        impl SingleScope for [< $i0:camel $( $i:camel )* >] {
            fn as_any(&self) -> &dyn Any {
                self
            }

            fn as_dyn(&self) -> DynSingleScope {
                DynSingleScope(&Self)
            }

            fn as_str(&self) -> &'static str {
                Self::STR
            }

            fn equals(&self, other: &dyn SingleScope) -> bool {
                other.as_any().downcast_ref::<Self>().is_some()
            }

            fn hash_value(&self) -> u64 {
                let mut hasher = ::std::hash::DefaultHasher::new();
                ::std::hash::Hash::hash(self, &mut hasher);
                ::std::hash::Hasher::finish(&hasher)
            }
        }

        impl Scope for [< $i0:camel $( $i:camel )* >] {
            fn scope(&self) -> HashSet<DynSingleScope> {
                [self.as_dyn()].into_iter().collect()
            }

            fn scope_str(&self) -> HashSet<&'static str> {
                [Self::STR].into_iter().collect()
            }

            fn grants(&self, other: &dyn SingleScope) -> bool {
                Self::STR == other.as_str()
            }

            fn boxed_clone(&self) -> BoxScope {
                box_scope!(*self)
            }
        }

        impl ::serde::Serialize for [< $i0:camel $( $i:camel )* >] {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: ::serde::Serializer,
            {
                serializer.serialize_str(Self::STR)
            }
        }

        impl<'de> ::serde::Deserialize<'de> for [< $i0:camel $( $i:camel )* >] {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: ::serde::de::Deserializer<'de>,
            {
                deserializer.deserialize_str([< $i0:camel $( $i:camel )* Visitor >])
            }
        }

        struct [< $i0:camel $( $i:camel )* Visitor >];

        impl<'de> ::serde::de::Visitor<'de> for [< $i0:camel $( $i:camel )* Visitor >] {
            type Value = [< $i0:camel $( $i:camel )* >];

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                write!(formatter, r#"a str "{}""#, [< $i0:camel $( $i:camel )* >]::STR)
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: ::serde::de::Error,
            {
                v.parse().map_err(E::custom)
            }
        }
    )+ } };
}

use scope;

// https://developers.google.com/identity/protocols/oauth2/scopes#calendar
scope! {
    calendar;
    calendar.readonly;
    calendar.events;
    calendar.events.readonly;
    calendar.settings.readonly;
    calendar.addons.execute;
}

/// ```
/// let combined = google_oauth::combine_scope![calendar, calendar.readonly];
/// # let _ = combined;
/// ```
#[macro_export]
macro_rules! combine_scope {
    [
        $hi0:ident $(. $hi:ident)*
        $(, $i0:ident $(. $i:ident)* )*
        $(,)?
    ] => { ::paste::paste! { {
        use $crate::scope::Scope;
        $crate::scope::[< $hi0:camel $( $hi:camel )* >] $(
            .with( $crate::scope::[< $i0:camel $( $i:camel )* >] )
        )*
    } } };
}
