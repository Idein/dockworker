use crate::response;
use std::env;
use std::io;
use thiserror::Error;

/// Type of general docker error response
#[derive(Debug, serde::Deserialize, Error)]
#[error("{message}")]
pub struct DockerError {
    pub message: String,
}

#[derive(Error, Debug)]
pub enum Error {
    #[error("io error")]
    Io(#[from] io::Error),
    #[error("envvar error")]
    Envvar(#[from] env::VarError),
    #[error("hyper error")]
    Hyper(#[from] hyper::Error),
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
