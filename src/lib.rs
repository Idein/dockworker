//! Docker
#![doc(html_root_url="https://ghmlee.github.io/rust-docker/doc")]

// Increase the compiler's recursion limit for the `error_chain` crate.
#![recursion_limit = "1024"]

#[macro_use]
extern crate error_chain;
extern crate hyper;
#[cfg(feature="openssl")]
extern crate openssl;
#[cfg(windows)]
extern crate named_pipe;
#[cfg(unix)]
extern crate unix_socket;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate serde;
extern crate url;

mod test;
mod util;
#[cfg(unix)]
mod unix;
mod options;
mod docker;
mod hyper_client;
pub mod errors;
pub mod container;
pub mod stats;
pub mod system;
pub mod image;
pub mod process;
pub mod filesystem;
pub mod version;

pub use docker::Docker;
pub use options::*;
