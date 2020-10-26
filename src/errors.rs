use crate::docker;
use crate::response;
pub use failure::ResultExt;
use failure::{Backtrace, Context, Fail};
use std::env;
use std::fmt;
use std::io;

pub type Result<T> = ::std::result::Result<T, Error>;

#[derive(Fail, Debug, Clone)]
pub enum ErrorKind {
    #[fail(display = "io error")]
    Io,
    #[fail(display = "envvar error")]
    Envvar,
    #[fail(display = "hyper error")]
    Hyper,
    #[fail(display = "json error")]
    Json,
    #[fail(display = "docker error")]
    Docker,
    #[fail(display = "response error")]
    Response,
    #[fail(display = "http error")]
    Http,
    #[fail(display = "invalid uri: {}", var)]
    InvalidUri { var: String },
    #[fail(display = "ssl error")]
    SSL,
    #[fail(display = "could not connect: {}", addr)]
    CouldNotConnect { addr: String },
    #[fail(display = "could not find DOCKER_CERT_PATH")]
    NoCertPath,
    #[fail(display = "parse error: {}", input)]
    ParseError { input: String },
    #[fail(display = "ssl support was disabled at compile time")]
    SslDisabled,
    #[fail(display = "unsupported scheme: {}", host)]
    UnsupportedScheme { host: String },
    #[fail(display = "poison error: {}", message)]
    Poison { message: String },
    #[fail(display = "unknown error: {}", message)]
    Unknown { message: String },
}

#[derive(Debug)]
pub struct Error {
    inner: Context<ErrorKind>,
}

impl Fail for Error {
    fn cause(&self) -> Option<&(dyn Fail + 'static)> {
        self.inner.cause()
    }

    fn backtrace(&self) -> Option<&Backtrace> {
        self.inner.backtrace()
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.inner, f)
    }
}

impl Error {
    pub fn new(inner: Context<ErrorKind>) -> Error {
        Error { inner }
    }

    pub fn kind(&self) -> &ErrorKind {
        self.inner.get_context()
    }
}

impl From<ErrorKind> for Error {
    fn from(kind: ErrorKind) -> Error {
        Error {
            inner: Context::new(kind),
        }
    }
}

impl From<Context<ErrorKind>> for Error {
    fn from(inner: Context<ErrorKind>) -> Error {
        Error { inner }
    }
}

impl From<io::Error> for Error {
    fn from(error: io::Error) -> Self {
        Error {
            inner: error.context(ErrorKind::Io),
        }
    }
}

impl From<env::VarError> for Error {
    fn from(error: env::VarError) -> Self {
        Error {
            inner: error.context(ErrorKind::Envvar),
        }
    }
}

impl From<hyper::Error> for Error {
    fn from(error: hyper::Error) -> Self {
        Error {
            inner: error.context(ErrorKind::Hyper),
        }
    }
}

impl From<::serde_json::Error> for Error {
    fn from(error: ::serde_json::Error) -> Self {
        Error {
            inner: error.context(ErrorKind::Json),
        }
    }
}

impl From<docker::DockerError> for Error {
    fn from(error: docker::DockerError) -> Self {
        Error {
            inner: error.context(ErrorKind::Docker),
        }
    }
}

impl From<response::Error> for Error {
    fn from(error: response::Error) -> Self {
        Error {
            inner: error.context(ErrorKind::Response),
        }
    }
}

impl From<http::Error> for Error {
    fn from(error: http::Error) -> Self {
        Error {
            inner: error.context(ErrorKind::Http),
        }
    }
}

#[cfg(feature = "openssl")]
impl From<native_tls::Error> for Error {
    fn from(error: native_tls::Error) -> Self {
        Error {
            inner: error.context(ErrorKind::SSL),
        }
    }
}

#[cfg(feature = "openssl")]
impl From<openssl::error::ErrorStack> for Error {
    fn from(error: openssl::error::ErrorStack) -> Self {
        Error {
            inner: error.context(ErrorKind::SSL),
        }
    }
}
