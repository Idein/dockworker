use crate::hyper_client::Response;
use http::HeaderMap;
use std::path::Path;
use std::result;

/// A http client
pub trait HttpClient {
    type Err: Send + 'static;

    fn get(&self, headers: &HeaderMap, path: &str) -> result::Result<Response, Self::Err>;

    fn head(&self, headers: &HeaderMap, path: &str) -> result::Result<http::HeaderMap, Self::Err>;

    fn post(
        &self,
        headers: &HeaderMap,
        path: &str,
        body: &str,
    ) -> result::Result<Response, Self::Err>;

    fn delete(&self, headers: &HeaderMap, path: &str) -> result::Result<Response, Self::Err>;

    fn post_file(
        &self,
        headers: &HeaderMap,
        path: &str,
        file: &Path,
    ) -> result::Result<Response, Self::Err>;

    fn put_file(
        &self,
        headers: &HeaderMap,
        path: &str,
        file: &Path,
    ) -> result::Result<Response, Self::Err>;
}

/// Access to inner HttpClient
pub trait HaveHttpClient {
    type Client: HttpClient;
    fn http_client(&self) -> &Self::Client;
}
