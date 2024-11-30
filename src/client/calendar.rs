// FIXME
#![allow(dead_code)]

use crate::WebClientSecret;

use super::{AuthorizedClient, InsufficientScopeError, TokenResponse};

macro_rules! contain_scope {
    ( [
        $( $i0:ident $(. $i:ident)* ),+
    ] in $s:expr ) => { ::paste::paste! { {
        use $crate::scope::{Scope, SingleScope};
        let scope = Scope::scope($s);
        $( scope.contains(&SingleScope::as_dyn( & $crate::scope::[< $i0:camel $($i:camel)* >] )) )&&+
    } } };
}

#[derive(Clone, Copy)]
pub struct CalendarClient<'a> {
    inner: &'a AuthorizedClient,
}

impl AuthorizedClient {
    #[inline]
    pub fn calendar(&self) -> CalendarClient<'_> {
        CalendarClient { inner: self }
    }
}

impl<'a> CalendarClient<'a> {
    pub const BASE_PATH: &'static str = "/calendar/v3";

    pub(crate) fn request(&self, method: http::Method, uri: &str) -> reqwest::RequestBuilder {
        let uri = format!("{}{}", Self::BASE_PATH, uri);
        self.inner.request(method, &uri)
    }

    #[inline]
    fn secret(&self) -> &WebClientSecret {
        &self.inner.secret
    }

    #[inline]
    fn token(&self) -> &TokenResponse {
        &self.inner.token
    }
}

mod calendar_list {
    use super::*;

    #[derive(Clone, Copy)]
    pub struct Client<'a> {
        pub(crate) inner: CalendarClient<'a>,
    }

    impl<'a> CalendarClient<'a> {
        #[inline]
        pub fn calendar_list(&self) -> Client<'a> {
            Client { inner: *self }
        }
    }

    impl<'a> Client<'a> {
        pub const BASE_PATH: &'static str = "/users/me/calendarList";

        pub(crate) fn request(&self, method: http::Method, uri: &str) -> reqwest::RequestBuilder {
            let uri = format!("{}{}", Self::BASE_PATH, uri);
            self.inner.request(method, &uri)
        }

        #[inline]
        fn secret(&self) -> &WebClientSecret {
            self.inner.secret()
        }

        #[inline]
        fn token(&self) -> &TokenResponse {
            self.inner.token()
        }

        pub fn list(&self) -> Result<list::Request<'a>, InsufficientScopeError> {
            if !contain_scope!([calendar, calendar.readonly] in &self.token().scope) {
                return Err(InsufficientScopeError::new());
            }
            Ok(list::Request::new(*self))
        }
    }

    mod list {
        use std::borrow::Cow;
        use std::fmt;
        use std::str::FromStr;

        use serde::{Deserialize, Serialize};

        use super::*;

        /// https://developers.google.com/calendar/api/v3/reference/calendarList/list
        #[derive(Clone)]
        pub struct Request<'a> {
            pub(crate) client: Client<'a>,
            pub(crate) parameters: Parameters,
        }

        impl<'a> Request<'a> {
            pub(super) fn new(client: Client<'a>) -> Self {
                Self {
                    client,
                    parameters: Parameters::new(),
                }
            }

            pub fn replace_parameters<F>(self, with: F) -> Self
            where
                F: FnOnce(Parameters) -> Parameters,
            {
                let Self { client, parameters } = self;
                Self {
                    client,
                    parameters: with(parameters),
                }
            }

            pub fn param_max_results(self, value: u8) -> Self {
                let Self { client, parameters } = self;
                Self {
                    client,
                    parameters: parameters.max_results(value),
                }
            }

            pub fn param_min_access_role(self, value: ParameterMinAccessRole) -> Self {
                let Self { client, parameters } = self;
                Self {
                    client,
                    parameters: parameters.min_access_role(value),
                }
            }

            pub fn param_page_token<'s, S>(self, value: S) -> Self
            where
                S: Into<Cow<'s, str>>,
            {
                let Self { client, parameters } = self;
                Self {
                    client,
                    parameters: parameters.page_token(value),
                }
            }

            pub fn param_show_deleted(self, value: bool) -> Self {
                let Self { client, parameters } = self;
                Self {
                    client,
                    parameters: parameters.show_deleted(value),
                }
            }

            pub fn param_show_hidden(self, value: bool) -> Self {
                let Self { client, parameters } = self;
                Self {
                    client,
                    parameters: parameters.show_hidden(value),
                }
            }

            pub fn param_sync_token<'s, S>(self, value: S) -> Self
            where
                S: Into<Cow<'s, str>>,
            {
                let Self { client, parameters } = self;
                Self {
                    client,
                    parameters: parameters.sync_token(value),
                }
            }

            pub async fn send(self) -> reqwest::Result<Response> {
                let Self { client, parameters } = self;
                let query = parameters.into_query();
                let uri = if query.is_empty() {
                    String::new()
                } else {
                    format!("?{}", query)
                };
                let res: Response = client
                    .request(http::Method::GET, &uri)
                    .send()
                    .await?
                    .json()
                    .await?;
                Ok(res)
            }
        }

        #[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
        pub struct Parameters {
            max_results: Option<u8>,
            min_access_role: Option<ParameterMinAccessRole>,
            page_token: Option<String>,
            show_deleted: bool,
            show_hidden: bool,
            sync_token: Option<String>,
        }

        impl Parameters {
            pub fn new() -> Self {
                Self::default()
            }

            pub fn max_results(self, value: u8) -> Self {
                Self {
                    max_results: Some(value),
                    ..self
                }
            }

            pub fn min_access_role(self, value: ParameterMinAccessRole) -> Self {
                Self {
                    min_access_role: Some(value),
                    ..self
                }
            }

            pub fn page_token<'a, S>(self, value: S) -> Self
            where
                S: Into<Cow<'a, str>>,
            {
                Self {
                    page_token: Some(value.into().into_owned()),
                    ..self
                }
            }

            pub fn show_deleted(self, value: bool) -> Self {
                Self {
                    show_deleted: value,
                    ..self
                }
            }

            pub fn show_hidden(self, value: bool) -> Self {
                Self {
                    show_hidden: value,
                    ..self
                }
            }

            pub fn sync_token<'a, S>(self, value: S) -> Self
            where
                S: Into<Cow<'a, str>>,
            {
                Self {
                    sync_token: Some(value.into().into_owned()),
                    ..self
                }
            }

            pub fn into_query(self) -> String {
                use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};

                let Self {
                    max_results,
                    min_access_role,
                    page_token,
                    show_deleted,
                    show_hidden,
                    sync_token,
                } = self;
                let params = [
                    max_results.map(|v| format!("maxResults={v}")),
                    min_access_role.map(|v| format!("minAccessRole={v}")),
                    page_token.map(|v| {
                        let encoded = utf8_percent_encode(&v, NON_ALPHANUMERIC);
                        format!("pageToken={encoded}")
                    }),
                    Some(format!("showDeleted={show_deleted}")),
                    Some(format!("showHidden={show_hidden}")),
                    sync_token.map(|v| {
                        let encoded = utf8_percent_encode(&v, NON_ALPHANUMERIC);
                        format!("syncToken={encoded}")
                    }),
                ];
                let params: Vec<String> = params.into_iter().flatten().collect();
                params.join("&")
            }
        }

        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[serde(rename_all = "camelCase")]
        pub enum ParameterMinAccessRole {
            FreeBusyReader,
            Owner,
            Reader,
            Writer,
        }

        impl ParameterMinAccessRole {
            pub fn as_str(&self) -> &'static str {
                match self {
                    Self::FreeBusyReader => "freeBusyReader",
                    Self::Owner => "owner",
                    Self::Reader => "reader",
                    Self::Writer => "writer",
                }
            }
        }

        impl FromStr for ParameterMinAccessRole {
            // FIXME
            type Err = &'static str;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                match s.trim() {
                    "freeBusyReader" => Ok(Self::FreeBusyReader),
                    "owner" => Ok(Self::Owner),
                    "reader" => Ok(Self::Reader),
                    "writer" => Ok(Self::Writer),
                    _ => Err("received invalid str"),
                }
            }
        }

        impl fmt::Display for ParameterMinAccessRole {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(self.as_str())
            }
        }

        // FIXME
        pub type Response = serde_json::Value;
    }
}
