use std::env;
use std::io;

use base64;
use docker;
use http;
use hyper;
#[cfg(feature = "openssl")]
use openssl;
use response;
use thiserror;

pub type Result<T> = ::std::result::Result<T, Error>;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("io error")]
    Io {
        #[from]
        source: io::Error,
    },
    #[error("envvar error")]
    Envvar {
        #[from]
        source: env::VarError,
    },
    #[error("hyper error")]
    Hyper {
        #[from]
        source: hyper::Error,
    },
    #[error("json error")]
    Json {
        #[from]
        source: ::serde_json::Error,
    },
    #[error("docker error")]
    Docker {
        #[from]
        source: docker::DockerError,
    },
    #[error("response error")]
    Response {
        #[from]
        source: response::Error,
    },
    #[error("http error")]
    Http {
        #[from]
        source: http::Error,
    },
    #[error("invalid uri")]
    InvalidUri {
        #[from]
        source: http::uri::InvalidUri,
    },
    #[error("could not connect: {addr:?}")]
    CouldNotConnect { addr: String, source: Box<Error> },
    #[error("ssl error")]
    SSL,
    #[error("could not find DOCKER_CERT_PATH")]
    NoCertPath,
    #[error("parse error: {input:?}")]
    ParseError {
        input: String,
        source: base64::DecodeError,
    },
    #[error("ssl support was disabled at compile time")]
    SslDisabled,
    #[error("unsupported scheme: {host:?}")]
    UnsupportedScheme { host: String },
    #[error("poison error: {message:?}")]
    Poison { message: String },
    #[error("unknown error: {message:?}")]
    Unknown { message: String },
}
