use std::collections::HashSet;

mod private {
    pub trait Sealed {}
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
}

macro_rules! box_scope {
    ($e:expr) => {
        BoxScope(Box::new($e))
    };
}

impl<S: SingleScope> Scope for S
where
    S: Clone + 'static,
{
    fn scope(&self) -> HashSet<&'static str> {
        [self.as_str()].into_iter().collect()
    }

    fn grants(&self, other: &dyn SingleScope) -> bool {
        let other = other.as_str();
        self.as_str() == other
    }

    fn boxed_clone(&self) -> BoxScope {
        box_scope!(self.clone())
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct Combine<S = NoScope> {
    inner: S,
}

impl Default for Combine<NoScope> {
    fn default() -> Self {
        Self::new()
    }
}

impl Combine<NoScope> {
    pub fn new() -> Self {
        Self { inner: NoScope }
    }
}

impl<S> Combine<S> {
    fn wrap_ref(s: &S) -> &Self {
        #[allow(unsafe_code)]
        unsafe {
            &*(s as *const S as *const Self)
        }
    }

    pub fn with<T: SingleScope>(self, single_scope: T) -> Combine<(T, S)> {
        let Self { inner } = self;
        Combine {
            inner: (single_scope, inner),
        }
    }

    pub fn into_boxed(self) -> BoxScope
    where
        Self: Scope + Clone + 'static,
    {
        box_scope!(self)
    }
}

impl<S, T> Combine<(S, T)>
where
    S: SingleScope,
    Combine<T>: Scope,
{
    fn unnest_scope(s: &S, t: &T) -> HashSet<&'static str> {
        let mut scope = Combine::wrap_ref(t).scope();
        scope.insert(s.as_str());
        scope
    }
}

impl<S> private::Sealed for Combine<S> {}

impl Scope for Combine<NoScope> {
    #[inline]
    fn scope(&self) -> HashSet<&'static str> {
        self.inner.scope()
    }

    #[inline]
    fn grants(&self, other: &dyn SingleScope) -> bool {
        self.inner.grants(other)
    }

    #[inline]
    fn boxed_clone(&self) -> BoxScope {
        box_scope!(*self)
    }
}

impl<S, T> Scope for Combine<(S, T)>
where
    S: SingleScope,
    T: Send + Sync + 'static,
    Combine<T>: Scope,
    Self: Clone,
{
    fn scope(&self) -> HashSet<&'static str> {
        let Self { inner: (s, t) } = self;
        Self::unnest_scope(s, t)
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

            pub fn with<S: SingleScope>(self, other: S) -> Combine<(S, (Self, NoScope))> {
                Combine::new().with(self).with(other)
            }
        }

        impl private::Sealed for [< $i0:camel $( $i:camel )* >] {}

        impl SingleScope for [< $i0:camel $( $i:camel )* >] {
            fn as_str(&self) -> &'static str {
                Self::STR
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
    [ $( $i0:ident $(. $i:ident)* ),* $(,)? ] => { ::paste::paste! {
        $crate::scope::Combine::new() $(
            .with( $crate::scope::[< $i0:camel $( $i:camel )* >] )
        )*
    } };
}
