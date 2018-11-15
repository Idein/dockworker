//! Docker Engine API client

// for the `error_chain` crate.
#![recursion_limit = "1024"]

extern crate base64;
extern crate byteorder;
#[macro_use]
extern crate error_chain;
extern crate hyper;
#[macro_use]
extern crate log;
#[cfg(windows)]
extern crate named_pipe;
extern crate nix;
#[cfg(feature = "openssl")]
extern crate openssl;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate tar;
#[cfg(unix)]
extern crate unix_socket;
extern crate url;

mod response;
mod header;
pub mod container;
mod docker;
pub mod errors;
pub mod filesystem;
mod hyper_client;
mod http_client;
pub mod image;
mod options;
pub mod process;
pub mod signal;
pub mod stats;
pub mod system;
mod test;
#[cfg(unix)]
mod unix;
mod util;
pub mod version;
pub mod credentials;

pub use docker::Docker;
pub use options::*;
