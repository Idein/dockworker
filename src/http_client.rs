use http::{HeaderMap, Response};
use std::path::Path;

/// A http client
#[async_trait::async_trait]
pub trait HttpClient {
    type Err: Send + 'static;

    async fn get(&self, headers: &HeaderMap, path: &str) -> Result<Response<Vec<u8>>, Self::Err>;

    async fn get_stream(
        &self,
        headers: &HeaderMap,
        path: &str,
    ) -> Result<Response<hyper::Body>, Self::Err>;

    async fn head(&self, headers: &HeaderMap, path: &str) -> Result<HeaderMap, Self::Err>;

    async fn post(
        &self,
        headers: &HeaderMap,
        path: &str,
        body: &str,
    ) -> Result<Response<Vec<u8>>, Self::Err>;

    async fn post_stream(
        &self,
        headers: &HeaderMap,
        path: &str,
        body: &str,
    ) -> Result<Response<hyper::Body>, Self::Err>;

    async fn post_file(
        &self,
        headers: &HeaderMap,
        path: &str,
        file: &Path,
    ) -> Result<Response<Vec<u8>>, Self::Err>;

    async fn post_file_stream(
        &self,
        headers: &HeaderMap,
        path: &str,
        file: &Path,
    ) -> Result<Response<hyper::Body>, Self::Err>;

    async fn delete(&self, headers: &HeaderMap, path: &str)
        -> Result<Response<Vec<u8>>, Self::Err>;

    async fn put_file(
        &self,
        headers: &HeaderMap,
        path: &str,
        file: &Path,
    ) -> Result<Response<Vec<u8>>, Self::Err>;
}

/// Access to inner HttpClient
pub trait HaveHttpClient {
    type Client: HttpClient;
    fn http_client(&self) -> &Self::Client;
}
