use std::{collections::HashSet, fmt};

mod private {
    pub trait Sealed {}
}

macro_rules! box_scope {
    ($e:expr) => {
        BoxScope(Box::new($e))
    };
}

pub trait SingleScope: private::Sealed + Send + Sync + 'static {
    fn as_str(&self) -> &'static str;
}

pub trait Scope: private::Sealed + Send + Sync + 'static {
    fn scope(&self) -> HashSet<&'static str>;

    fn grants(&self, other: &dyn SingleScope) -> bool {
        let other = other.as_str();
        self.scope().contains(other)
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

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct NoScope;

impl private::Sealed for NoScope {}

impl Scope for NoScope {
    #[inline]
    fn scope(&self) -> HashSet<&'static str> {
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
    fn scope(&self) -> HashSet<&'static str> {
        self.0.scope()
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
        f.debug_tuple("BoxScope").field(&self.scope()).finish()
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
    fn scope(&self) -> HashSet<&'static str> {
        let Self(a, b) = self;
        let mut scope_a = a.scope();
        let scope_b = b.scope();
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

        impl private::Sealed for [< $i0:camel $( $i:camel )* >] {}

        impl SingleScope for [< $i0:camel $( $i:camel )* >] {
            fn as_str(&self) -> &'static str {
                Self::STR
            }
        }

        impl Scope for [< $i0:camel $( $i:camel )* >] {
            fn scope(&self) -> HashSet<&'static str> {
                [Self::STR].into_iter().collect()
            }

            fn grants(&self, other: &dyn SingleScope) -> bool {
                Self::STR == other.as_str()
            }

            fn boxed_clone(&self) -> BoxScope {
                box_scope!(*self)
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
