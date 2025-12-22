use std::env;
use std::io;

use thiserror::Error;

use crate::response;

/// Type of general docker error response
#[derive(Debug, serde::Deserialize, Error)]
#[error("{message}")]
pub struct DockerError {
    pub message: String,
}

#[derive(Error, Debug)]
pub enum Error {
    #[error("io error")]
    Io(io::Error),
    #[error("envvar error")]
    Envvar(#[from] env::VarError),
    #[error("hyper error")]
    Hyper(hyper::Error),
    #[error("connection refused")]
    ConnectionRefused(Box<dyn StdError + Send + Sync>),
    #[error("connection reset")]
    ConnectionReset(Box<dyn StdError + Send + Sync>),
    #[error("json error")]
    Json(#[from] serde_json::Error),
    #[error("docker error")]
    Docker(#[from] DockerError),
    #[error("response error")]
    Response(#[from] response::Error),
    #[error("http error")]
    Http(#[from] http::Error),
    #[error("invalid uri")]
    InvalidUri {
        var: String,
        source: http::uri::InvalidUri,
    },
    #[cfg(feature = "native-tls")]
    #[error("ssl error")]
    NativeTls(#[from] native_tls::Error),
    #[cfg(feature = "openssl")]
    #[error("ssl error")]
    OpenSsl(#[from] openssl::error::ErrorStack),
    #[cfg(feature = "rustls")]
    #[error("ssl error")]
    Rustls(#[from] rustls::Error),
    #[error("could not connect: {}", addr)]
    CouldNotConnect { addr: String, source: Box<Error> },
    #[error("could not find DOCKER_CERT_PATH")]
    NoCertPath,
    #[error("parse error: {}", input)]
    ParseError {
        input: String,
        source: base64::DecodeError,
    },
    #[error("ssl support was disabled at compile time")]
    SslDisabled,
    #[error("unsupported scheme: {}", host)]
    UnsupportedScheme { host: String },
    #[error("poison error: {}", message)]
    Poison { message: String },
    #[error("unknown error: {}", message)]
    Unknown { message: String },
}

impl From<hyper::Error> for Error {
    fn from(err: hyper::Error) -> Self {
        if err.is_connect() {
            use std::error::Error as _;
            return match err
                .source()
                .and_then(|e| e.downcast_ref::<io::Error>())
                .map(|e| e.kind())
            {
                io::ErrorKind::ConnectionRefused => Error::ConnectionRefused(Box::new(err)),
                io::ErrorKind::ConnectionReset => Error::ConnectionReset(Box::new(err)),
                _ => Error::Hyper(err),
            };
        }
        return Error::Hyper(err);
    }
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Self {
        match err.kind() {
            io::ErrorKind::ConnectionRefused => {
                return Error::ConnectionRefused(Box::new(err));
            }
            io::ErrorKind::ConnectionReset => {
                return Error::ConnectionReset(Box::new(err));
            }
            _ => return Error::Io(err),
        }
    }
}
