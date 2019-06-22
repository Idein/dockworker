//! Docker Engine API client

// for the `error_chain` crate.
#![recursion_limit = "1024"]

extern crate base64;
extern crate byteorder;
extern crate chrono;
#[macro_use]
extern crate failure;
extern crate futures;
extern crate http;
extern crate hyper;
#[cfg(feature = "openssl")]
extern crate hyper_tls;
#[cfg(unix)]
extern crate hyperlocal;
extern crate hyperx;
#[macro_use]
extern crate log;
extern crate mime;
#[cfg(windows)]
extern crate named_pipe;
#[cfg(feature = "openssl")]
extern crate native_tls;
extern crate nix;
#[cfg(feature = "openssl")]
extern crate openssl;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate dirs;
extern crate serde_json;
extern crate tar;
extern crate tokio;
#[cfg(unix)]
extern crate unix_socket;
extern crate url;

pub mod container;
pub mod credentials;
mod docker;
pub mod errors;
pub mod filesystem;
mod header;
mod http_client;
mod hyper_client;
pub mod image;
mod options;
pub mod process;
mod response;
pub mod signal;
pub mod stats;
pub mod system;
mod test;
pub mod version;

pub use docker::Docker;
pub use options::*;
