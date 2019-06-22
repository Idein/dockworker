use std::path::Path;
use std::result;

use hyper_client::*;

/// A http client
pub trait HttpClient {
    type Err: Send + 'static;

    fn get(&self, headers: &Headers, path: &str) -> result::Result<Response, Self::Err>;

    fn post(
        &self,
        headers: &Headers,
        path: &str,
        body: &str,
    ) -> result::Result<Response, Self::Err>;

    fn delete(&self, headers: &Headers, path: &str) -> result::Result<Response, Self::Err>;

    fn post_file(
        &self,
        headers: &Headers,
        path: &str,
        file: &Path,
    ) -> result::Result<Response, Self::Err>;

    fn put_file(
        &self,
        headers: &Headers,
        path: &str,
        file: &Path,
    ) -> result::Result<Response, Self::Err>;
}

/// Access to inner HttpClient
pub trait HaveHttpClient {
    type Client: HttpClient;
    fn http_client(&self) -> &Self::Client;
}
