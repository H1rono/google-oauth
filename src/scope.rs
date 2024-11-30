use std::any::Any;
use std::borrow::Borrow;
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::hash::{Hash, Hasher};
use std::str::FromStr;
use std::sync::LazyLock;

mod serde;

mod private {
    pub trait Sealed {}
}

macro_rules! box_scope {
    ($e:expr) => {
        BoxScope(Box::new($e))
    };
}

pub trait SingleScope: private::Sealed + fmt::Debug + Send + Sync + 'static {
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

    fn space_delimited(&self) -> SpaceDelimitedScope {
        self.scope().into_iter().collect::<Vec<_>>().into()
    }

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

#[derive(Debug, Clone, Copy)]
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

impl FromStr for DynSingleScope {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let Some(s) = ALL_SCOPE_MAP.get(s) else {
            return Err("no matching scope found");
        };
        Ok(*s)
    }
}

impl fmt::Display for DynSingleScope {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
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
    #[inline]
    fn as_any(&self) -> &dyn Any {
        self.0.as_any()
    }

    #[inline]
    fn as_dyn(&self) -> DynSingleScope {
        *self
    }

    #[inline]
    fn as_str(&self) -> &'static str {
        self.0.as_str()
    }

    #[inline]
    fn equals(&self, other: &dyn SingleScope) -> bool {
        self.0.equals(other)
    }

    #[inline]
    fn hash_value(&self) -> u64 {
        self.0.hash_value()
    }
}

impl Scope for DynSingleScope {
    fn scope(&self) -> HashSet<DynSingleScope> {
        [*self].into()
    }

    fn scope_str(&self) -> HashSet<&'static str> {
        [self.as_str()].into()
    }

    fn boxed_clone(&self) -> BoxScope {
        box_scope!(*self)
    }

    fn space_delimited(&self) -> SpaceDelimitedScope {
        vec![*self].into()
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

    #[inline]
    fn space_delimited(&self) -> SpaceDelimitedScope {
        Default::default()
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct SpaceDelimitedScope(Vec<DynSingleScope>);

impl AsRef<[DynSingleScope]> for SpaceDelimitedScope {
    fn as_ref(&self) -> &[DynSingleScope] {
        &self.0
    }
}

impl Borrow<[DynSingleScope]> for SpaceDelimitedScope {
    fn borrow(&self) -> &[DynSingleScope] {
        &self.0
    }
}

impl FromStr for SpaceDelimitedScope {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let inner = s
            .split(' ')
            .map(|s| s.parse())
            .collect::<Result<Vec<DynSingleScope>, _>>()
            .map_err(|e| e.to_string())?;
        Ok(inner.into())
    }
}

impl fmt::Display for SpaceDelimitedScope {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Some((first, tail)) = self.0.split_first() else {
            return Ok(());
        };
        write!(f, "{first}")?;
        for s in tail {
            write!(f, " {s}")?;
        }
        Ok(())
    }
}

impl From<Vec<DynSingleScope>> for SpaceDelimitedScope {
    fn from(value: Vec<DynSingleScope>) -> Self {
        Self(value)
    }
}

impl private::Sealed for SpaceDelimitedScope {}

impl Scope for SpaceDelimitedScope {
    fn scope(&self) -> HashSet<DynSingleScope> {
        self.0.iter().copied().collect()
    }

    fn scope_str(&self) -> HashSet<&'static str> {
        self.0.iter().map(SingleScope::as_str).collect()
    }

    fn boxed_clone(&self) -> BoxScope {
        box_scope!(self.clone())
    }

    #[inline]
    fn space_delimited(&self) -> SpaceDelimitedScope {
        self.clone()
    }
}

pub struct BoxScope(Box<dyn Scope>);

impl private::Sealed for BoxScope {}

impl Scope for BoxScope {
    #[inline]
    fn scope(&self) -> HashSet<DynSingleScope> {
        self.0.scope()
    }

    #[inline]
    fn scope_str(&self) -> HashSet<&'static str> {
        self.0.scope_str()
    }

    #[inline]
    fn boxed_clone(&self) -> BoxScope {
        self.0.boxed_clone()
    }

    #[inline]
    fn space_delimited(&self) -> SpaceDelimitedScope {
        self.0.space_delimited()
    }

    #[inline]
    fn into_boxed(self) -> BoxScope
    where
        Self: Sized,
    {
        self
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
        let mut scope_b = b.scope();
        scope_a.extend(scope_b.drain());
        scope_a
    }

    fn scope_str(&self) -> HashSet<&'static str> {
        let Self(a, b) = self;
        let mut scope_a = a.scope_str();
        let mut scope_b = b.scope_str();
        scope_a.extend(scope_b.drain());
        scope_a
    }

    fn boxed_clone(&self) -> BoxScope {
        box_scope!(self.clone())
    }

    fn space_delimited(&self) -> SpaceDelimitedScope {
        let Self(a, b) = self;
        let SpaceDelimitedScope(mut scope_a) = a.space_delimited();
        let SpaceDelimitedScope(mut scope_b) = b.space_delimited();
        scope_a.append(&mut scope_b);
        scope_a.into()
    }
}

macro_rules! scope {
    { $(
        $( #[$m:meta] )*
        $i0:ident $(. $i:ident)* ;
    )+ } => { ::paste::paste! { $(
        $( #[$m:meta] )*
        #[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
        pub struct [< $i0:camel $( $i:camel )* >];

        impl [< $i0:camel $( $i:camel )* >] {
            pub const STR: &'static str = concat!(
                "https://www.googleapis.com/auth/",
                stringify!($i0)
                $(, ".", stringify!($i))*
            );

            pub const fn new() -> Self {
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
            #[inline]
            fn as_any(&self) -> &dyn Any {
                self
            }

            #[inline]
            fn as_dyn(&self) -> DynSingleScope {
                DynSingleScope(&Self)
            }

            #[inline]
            fn as_str(&self) -> &'static str {
                Self::STR
            }

            fn equals(&self, other: &dyn SingleScope) -> bool {
                other.as_any().downcast_ref::<Self>().is_some()
            }

            fn hash_value(&self) -> u64 {
                let mut hasher = ::std::hash::DefaultHasher::new();
                self.hash(&mut hasher);
                hasher.finish()
            }
        }

        impl Scope for [< $i0:camel $( $i:camel )* >] {
            fn scope(&self) -> HashSet<DynSingleScope> {
                [self.as_dyn()].into()
            }

            fn scope_str(&self) -> HashSet<&'static str> {
                [Self::STR].into()
            }

            fn grants(&self, other: &dyn SingleScope) -> bool {
                Self::STR == other.as_str()
            }

            fn boxed_clone(&self) -> BoxScope {
                box_scope!(*self)
            }

            fn space_delimited(&self) -> SpaceDelimitedScope {
                vec![self.as_dyn()].into()
            }
        }
    )+ } };
}

// https://developers.google.com/identity/protocols/oauth2/scopes#calendar
scope! {
    calendar;
    calendar.readonly;
    calendar.events;
    calendar.events.readonly;
    calendar.settings.readonly;
    calendar.addons.execute;
}

macro_rules! apply_all_scope {
    ($m:ident) => {
        $m! {
            calendar,
            calendar.readonly,
            calendar.events,
            calendar.events.readonly,
            calendar.settings.readonly,
            calendar.addons.execute
        }
    };
}

use {apply_all_scope, scope};

macro_rules! scope_pairs {
    [ $(
        $i0:ident $(. $i:ident )*
    ),* ] => { ::paste::paste! { [ $(
        ([< $i0:camel $($i:camel)* >]::STR, DynSingleScope(& [< $i0:camel $($i:camel)* >] ))
    ),* ] } };
}

pub const ALL_SCOPE_PAIRS: &[(&str, DynSingleScope)] = &apply_all_scope!(scope_pairs);

fn all_scope_map() -> HashMap<&'static str, DynSingleScope> {
    ALL_SCOPE_PAIRS.iter().copied().collect()
}

pub static ALL_SCOPE_MAP: LazyLock<HashMap<&'static str, DynSingleScope>> =
    LazyLock::new(all_scope_map);

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calendar_ser() {
        let payload = format!(r#""{}""#, Calendar::STR);
        let ser = serde_json::to_string(&Calendar).unwrap();
        assert_eq!(ser, payload);
    }

    #[test]
    fn test_calendar_de() {
        let payload = format!(r#""{}""#, Calendar::STR);
        let scope: Calendar = serde_json::from_str(&payload).unwrap();
        assert_eq!(scope, Calendar);
    }

    #[test]
    fn test_dyn_single_scope_ser() {
        let payload = format!(r#""{}""#, Calendar::STR);
        let scope = Calendar.as_dyn();
        let ser = serde_json::to_string(&scope).unwrap();
        assert_eq!(ser, payload);
    }

    #[test]
    fn test_dyn_single_scope_de() {
        let payload = format!(r#""{}""#, Calendar::STR);
        let scope: DynSingleScope = serde_json::from_str(&payload).unwrap();
        assert_eq!(scope, Calendar.as_dyn());
    }

    #[test]
    fn test_space_delimited_scope_ser() {
        let payload = format!(
            r#""{} {} {} {}""#,
            Calendar, CalendarReadonly, CalendarEvents, CalendarEventsReadonly
        );
        let scope: SpaceDelimitedScope = vec![
            Calendar.as_dyn(),
            CalendarReadonly.as_dyn(),
            CalendarEvents.as_dyn(),
            CalendarEventsReadonly.as_dyn(),
        ]
        .into();
        let ser = serde_json::to_string(&scope).unwrap();
        assert_eq!(ser, payload);
    }

    #[test]
    fn test_space_delimited_scope_de() {
        let payload = format!(
            r#""{} {} {} {}""#,
            Calendar, CalendarReadonly, CalendarEvents, CalendarEventsReadonly
        );
        let scope: SpaceDelimitedScope = vec![
            Calendar.as_dyn(),
            CalendarReadonly.as_dyn(),
            CalendarEvents.as_dyn(),
            CalendarEventsReadonly.as_dyn(),
        ]
        .into();
        let de: SpaceDelimitedScope = serde_json::from_str(&payload).unwrap();
        assert_eq!(de, scope);
    }
}
