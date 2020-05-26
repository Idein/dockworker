use std::fs::File;
use std::path::Path;

use http::header::HeaderMap;
use http::Request;
pub use http::StatusCode;
use hyper::rt::Stream;
use hyper::Uri;
pub use hyperx::header::{ContentType, Headers};

use errors::{Error, Result};
use http_client::HttpClient;

use std::io::Read;
use std::str::FromStr;

use futures::future::Future;
use futures::future::IntoFuture;
use tokio;

#[derive(Clone, Debug)]
enum Client {
    HttpClient(hyper::Client<hyper::client::HttpConnector>),
    #[cfg(feature = "openssl")]
    HttpsClient(hyper::Client<hyper_tls::HttpsConnector<hyper::client::HttpConnector>>),
    #[cfg(unix)]
    UnixClient(hyper::Client<hyperlocal::UnixConnector>),
}

impl Client {
    fn request(&self, req: Request<hyper::Body>) -> hyper::client::ResponseFuture {
        match self {
            Client::HttpClient(http_client) => http_client.request(req),
            #[cfg(feature = "openssl")]
            Client::HttpsClient(https_client) => https_client.request(req),
            #[cfg(unix)]
            Client::UnixClient(unix_client) => unix_client.request(req),
        }
    }
}

#[derive(Debug)]
pub struct Response {
    pub status: http::StatusCode,
    buf: Vec<u8>,
    rx: futures::sync::mpsc::UnboundedReceiver<hyper::Chunk>,
    handle: std::thread::JoinHandle<()>,
}

impl Response {
    pub fn new(mut res: http::Response<hyper::Body>) -> Response {
        let status = res.status();
        let (tx, rx) = futures::sync::mpsc::unbounded();

        let handle = std::thread::spawn(move || {
            let mut tokio_runtime = tokio::runtime::current_thread::Runtime::new().unwrap();

            let future = res.body_mut().for_each(move |chunk| {
                if !tx.is_closed() {
                    tx.unbounded_send(chunk).unwrap();
                }
                Ok(())
            });

            tokio_runtime.block_on(future).unwrap();
        });

        Response {
            status: status,
            buf: Vec::new(),
            rx: rx,
            handle: handle,
        }
    }
}

impl std::io::Read for Response {
    fn read(&mut self, buf: &mut [u8]) -> std::result::Result<usize, std::io::Error> {
        let n = buf.len();
        let m = std::cmp::min(self.buf.len(), n);
        let mut i = 0;

        for byte in self.buf.drain(..m) {
            buf[i] = byte;
            i += 1;
        }

        if n == m {
            return Ok(i);
        }

        let mut j = i;
        let mut buffer = Vec::new();

        {
            let stream = self
                .rx
                .by_ref()
                .map(|chunk| chunk.into_bytes())
                .skip_while(|bytes| {
                    let m = std::cmp::min(bytes.len(), n - j);
                    let len = bytes.len();
                    j += len;

                    for byte in &bytes[..m] {
                        buf[i] = *byte;
                        i += 1;
                    }

                    if len < m {
                        return Ok(true);
                    }

                    if len == m {
                        return Ok(false);
                    }

                    for byte in &bytes[m..] {
                        buffer.push(*byte);
                    }

                    Ok(false)
                });

            if let Err(_) = stream.into_future().wait() {
                unreachable!();
            }
        }

        self.buf = buffer;

        Ok(i)
    }
}

/// Http client using hyper
#[derive(Debug)]
pub struct HyperClient {
    /// http client
    client: Client,
    /// base connection address
    base: Uri,
    tokio_runtime: std::sync::Mutex<tokio::runtime::Runtime>,
}

fn join_uri(uri: &Uri, path: &str) -> Result<Uri> {
    let joined = format!("{}{}", uri.to_string(), path);
    Ok(Uri::from_str(&joined)?)
}

fn request_builder(method: &http::Method, uri: &Uri, headers: &Headers) -> http::request::Builder {
    let mut request = Request::builder();
    request.method(method);
    request.uri(uri);
    for header in headers.iter() {
        request.header(header.name(), header.value_string());
    }
    request
}

fn with_redirect<T: Into<hyper::Body> + Sync + Send + 'static + Clone>(
    max_redirects: u64,
    client: Client,
    method: http::Method,
    uri: Uri,
    headers: Headers,
    body: Option<T>,
    future: hyper::client::ResponseFuture,
) -> Box<
    dyn hyper::rt::Future<Item = hyper::Response<hyper::Body>, Error = hyper::error::Error>
        + Send
        + 'static,
> {
    if max_redirects == 0 {
        Box::new(future)
            as Box<
                dyn hyper::rt::Future<
                        Item = hyper::Response<hyper::Body>,
                        Error = hyper::error::Error,
                    > + Send
                    + 'static,
            >
    } else {
        Box::new(future.and_then(move |res| {
            let mut request = request_builder(&method, &uri, &headers);
            let uri_parts = http::uri::Parts::from(uri.clone());

            if !res.status().is_redirection() || res.headers().get("Location").is_none() {
                Box::new(Ok(res).into_future())
                    as Box<
                        dyn hyper::rt::Future<
                                Item = hyper::Response<hyper::Body>,
                                Error = hyper::error::Error,
                            > + Send
                            + 'static,
                    >
            } else {
                let mut see_other = false;

                if res.status() == hyper::StatusCode::SEE_OTHER {
                    request.method(hyper::Method::GET);
                    see_other = true;
                }

                let location = res.headers().get("Location").unwrap();
                let location = location.to_str().unwrap();
                let location = Uri::from_str(location).unwrap();
                let mut location_parts = http::uri::Parts::from(location);
                if location_parts.scheme.is_none() {
                    location_parts.scheme = uri_parts.scheme;
                }
                if location_parts.authority.is_none() {
                    location_parts.authority = uri_parts.authority;
                }
                let location = http::uri::Uri::from_parts(location_parts).unwrap();
                request.uri(location.clone());

                let future = client.request(if see_other {
                    request.body(hyper::Body::empty()).unwrap()
                } else {
                    if let Some(body) = body.clone() {
                        request.body(body.into()).unwrap()
                    } else {
                        request.body(hyper::Body::empty()).unwrap()
                    }
                });

                with_redirect(
                    max_redirects - 1,
                    client,
                    method,
                    uri,
                    headers,
                    body,
                    future,
                )
            }
        }))
            as Box<
                dyn hyper::rt::Future<
                        Item = hyper::Response<hyper::Body>,
                        Error = hyper::error::Error,
                    > + Send
                    + 'static,
            >
    }
}

fn request_with_redirect<T: Into<hyper::Body> + Sync + Send + 'static + Clone>(
    client: Client,
    method: http::Method,
    uri: Uri,
    headers: Headers,
    body: Option<T>,
) -> Result<
    Box<
        dyn hyper::rt::Future<Item = hyper::Response<hyper::Body>, Error = hyper::error::Error>
            + Send
            + 'static,
    >,
> {
    let request =
        request_builder(&method, &uri, &headers).body(if let Some(body) = body.clone() {
            body.into()
        } else {
            hyper::Body::empty()
        })?;
    let future = client.request(request);
    Ok(with_redirect(
        10, client, method, uri, headers, body, future,
    ))
}

impl HyperClient {
    fn new(client: Client, base: Uri) -> Self {
        Self {
            client,
            base,
            tokio_runtime: std::sync::Mutex::new(tokio::runtime::Runtime::new().unwrap()),
        }
    }

    /// path to unix socket
    #[cfg(unix)]
    pub fn connect_with_unix(path: &str) -> Self {
        let url = hyperlocal::Uri::new(path, "").into();
        let unix = hyperlocal::UnixConnector::new();
        let client = hyper::Client::builder().build::<_, hyper::Body>(unix);
        Self::new(Client::UnixClient(client), url)
    }

    #[cfg(feature = "openssl")]
    pub fn connect_with_ssl(addr: &str, key: &Path, cert: &Path, ca: &Path) -> Result<Self, Error> {
        let mut key_buf = Vec::new();
        let mut cert_buf = Vec::new();
        let mut ca_buf = Vec::new();

        let mut key_file = File::open(key)?;
        let mut cert_file = File::open(cert)?;
        let mut ca_file = File::open(ca)?;

        key_file.read_to_end(&mut key_buf)?;
        cert_file.read_to_end(&mut cert_buf)?;
        ca_file.read_to_end(&mut ca_buf)?;

        let pkey =
            openssl::pkey::PKey::from_rsa(openssl::rsa::Rsa::private_key_from_pem(&key_buf)?)?;
        let cert = openssl::x509::X509::from_pem(&cert_buf)?;
        let pkcs12 = openssl::pkcs12::Pkcs12::builder().build("", "", &pkey, &cert)?;
        let der = pkcs12.to_der()?;
        let id = native_tls::Identity::from_pkcs12(&der, "")?;
        let ca = native_tls::Certificate::from_pem(&ca_buf)?;
        let mut builder = native_tls::TlsConnector::builder();
        builder.identity(id);
        builder.add_root_certificate(ca);
        // This ensures that using docker-machine-esque addresses work with Hyper.
        let addr_https = addr.clone().replacen("tcp://", "https://", 1);
        let url = Uri::from_str(&addr_https)?;
        let mut http = hyper::client::HttpConnector::new(4);
        http.enforce_http(false);
        let https = hyper_tls::HttpsConnector::from((http, builder.build()?));
        let client = hyper::Client::builder().build::<_, hyper::Body>(https);
        Ok(Self::new(Client::HttpsClient(client), url))
    }

    pub fn connect_with_http(addr: &str) -> Result<Self> {
        // This ensures that using docker-machine-esque addresses work with Hyper.
        let addr_https = addr.clone().replace("tcp://", "http://");
        let url = Uri::from_str(&addr_https)?;
        Ok(Self::new(Client::HttpClient(hyper::Client::new()), url))
    }
}

impl HttpClient for HyperClient {
    type Err = Error;

    fn get(&self, headers: &Headers, path: &str) -> Result<Response> {
        let url = join_uri(&self.base, path)?;

        let res = self
            .tokio_runtime
            .lock()
            .unwrap()
            .block_on(request_with_redirect::<Vec<u8>>(
                self.client.clone(),
                http::Method::GET,
                url,
                headers.clone(),
                None,
            )?)?;

        Ok(Response::new(res))
    }

    fn head(&self, headers: &Headers, path: &str) -> Result<HeaderMap> {
        let url = join_uri(&self.base, path)?;

        let res = self
            .tokio_runtime
            .lock()
            .unwrap()
            .block_on(request_with_redirect::<Vec<u8>>(
                self.client.clone(),
                http::Method::HEAD,
                url,
                headers.clone(),
                None,
            )?)?;

        Ok(res.headers().clone())
    }

    fn post(&self, headers: &Headers, path: &str, body: &str) -> Result<Response> {
        let url = join_uri(&self.base, path)?;

        let res = self
            .tokio_runtime
            .lock()
            .unwrap()
            .block_on(request_with_redirect(
                self.client.clone(),
                http::Method::POST,
                url,
                headers.clone(),
                Some(body.to_string()),
            )?)?;

        Ok(Response::new(res))
    }

    fn delete(&self, headers: &Headers, path: &str) -> Result<Response> {
        let url = join_uri(&self.base, path)?;

        let res = self
            .tokio_runtime
            .lock()
            .unwrap()
            .block_on(request_with_redirect::<Vec<u8>>(
                self.client.clone(),
                http::Method::DELETE,
                url,
                headers.clone(),
                None,
            )?)?;

        Ok(Response::new(res))
    }

    fn post_file(&self, headers: &Headers, path: &str, file: &Path) -> Result<Response> {
        let mut content = File::open(file)?;
        let url = join_uri(&self.base, path)?;

        let mut buf = Vec::new();
        content.read_to_end(&mut buf)?;

        let res = self
            .tokio_runtime
            .lock()
            .unwrap()
            .block_on(request_with_redirect(
                self.client.clone(),
                http::Method::POST,
                url,
                headers.clone(),
                Some(buf),
            )?)?;

        Ok(Response::new(res))
    }

    fn put_file(&self, headers: &Headers, path: &str, file: &Path) -> Result<Response> {
        let mut content = File::open(file)?;
        let url = join_uri(&self.base, path)?;

        let mut buf = Vec::new();
        content.read_to_end(&mut buf)?;

        let res = self
            .tokio_runtime
            .lock()
            .unwrap()
            .block_on(request_with_redirect(
                self.client.clone(),
                http::Method::PUT,
                url,
                headers.clone(),
                Some(buf),
            )?)?;

        Ok(Response::new(res))
    }
}
