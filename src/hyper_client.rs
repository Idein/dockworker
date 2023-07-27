use crate::errors::Error as DwError;
use crate::http_client::HttpClient;
use http::{HeaderMap, Request, Response};
use hyper::Uri;
use std::path::Path;
use std::str::FromStr;

#[allow(clippy::enum_variant_names)]
#[derive(Clone, Debug)]
enum Client {
    HttpClient(hyper::Client<hyper::client::HttpConnector>),
    #[cfg(feature = "openssl")]
    HttpsClient(hyper::Client<hyper_tls::HttpsConnector<hyper::client::HttpConnector>>),
    #[cfg(feature = "rustls")]
    HttpsClient(hyper::Client<hyper_rustls::HttpsConnector<hyper::client::HttpConnector>>),
    #[cfg(unix)]
    UnixClient(hyper::Client<hyperlocal::UnixConnector>),
}

impl Client {
    fn request(&self, req: Request<hyper::Body>) -> hyper::client::ResponseFuture {
        match self {
            Client::HttpClient(http_client) => http_client.request(req),
            #[cfg(feature = "openssl")]
            Client::HttpsClient(https_client) => https_client.request(req),
            #[cfg(feature = "rustls")]
            Client::HttpsClient(https_client) => https_client.request(req),
            #[cfg(unix)]
            Client::UnixClient(unix_client) => unix_client.request(req),
        }
    }
}

/// Http client using hyper
#[derive(Debug, Clone)]
pub struct HyperClient {
    /// http client
    client: Client,
    /// base connection address
    base: Uri,
}

fn join_uri(uri: &Uri, path: &str) -> Result<Uri, DwError> {
    let joined = format!("{uri}{path}");
    Uri::from_str(&joined).map_err(|err| DwError::InvalidUri {
        var: joined,
        source: err,
    })
}

fn request_builder(
    method: &http::Method,
    uri: &Uri,
    headers: &HeaderMap,
) -> http::request::Builder {
    let mut request = Request::builder().method(method).uri(uri);
    for (name, value) in headers.iter() {
        request = request.header(name, value);
    }
    request
}

async fn request_with_redirect<T: Into<hyper::Body> + Sync + Send + 'static + Clone>(
    client: Client,
    method: http::Method,
    uri: Uri,
    headers: HeaderMap,
    body: Option<T>,
) -> Result<http::Response<hyper::Body>, DwError> {
    let request =
        request_builder(&method, &uri, &headers).body(if let Some(body) = body.clone() {
            body.into()
        } else {
            hyper::Body::empty()
        })?;
    let mut future = client.request(request);
    let mut max_redirects = 10;
    loop {
        let resp = future.await?;
        if max_redirects == 0 {
            return Ok(resp);
        } else {
            let mut request = request_builder(&method, &uri, &headers);
            let uri_parts = http::uri::Parts::from(uri.clone());

            if !resp.status().is_redirection() || resp.headers().get("Location").is_none() {
                return Ok(resp);
            } else {
                let mut see_other = false;

                if resp.status() == hyper::StatusCode::SEE_OTHER {
                    request = request.method(hyper::Method::GET);
                    see_other = true;
                }

                let location = resp.headers().get("Location").unwrap();
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
                request = request.uri(location.clone());

                future = client.request(if see_other {
                    request.body(hyper::Body::empty()).unwrap()
                } else if let Some(body) = body.clone() {
                    request.body(body.into()).unwrap()
                } else {
                    request.body(hyper::Body::empty()).unwrap()
                });

                max_redirects -= 1;
            }
        }
    }
}

async fn fetch_body(resp: http::Response<hyper::Body>) -> Result<http::Response<Vec<u8>>, DwError> {
    let (p, b) = resp.into_parts();
    let b = hyper::body::to_bytes(b).await?.to_vec();
    Ok(Response::from_parts(p, b))
}

impl HyperClient {
    fn new(client: Client, base: Uri) -> Self {
        Self { client, base }
    }

    /// path to unix socket
    #[cfg(unix)]
    pub fn connect_with_unix(path: &str) -> Self {
        let url = hyperlocal::Uri::new(path, "").into();
        // Prevent from using connection pooling.
        // See https://github.com/hyperium/hyper/issues/2312.
        let client: hyper::Client<_> = hyper::Client::builder()
            .pool_idle_timeout(std::time::Duration::from_millis(0))
            .pool_max_idle_per_host(0)
            .build(hyperlocal::UnixConnector);
        Self::new(Client::UnixClient(client), url)
    }

    #[cfg(feature = "openssl")]
    pub fn connect_with_ssl(
        addr: &str,
        key: &Path,
        cert: &Path,
        ca: &Path,
    ) -> Result<Self, DwError> {
        let key_buf = std::fs::read(key)?;
        let cert_buf = std::fs::read(cert)?;
        let ca_buf = std::fs::read(ca)?;

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
        let addr_https = addr.to_string().replacen("tcp://", "https://", 1);
        let url = Uri::from_str(&addr_https).map_err(|err| DwError::InvalidUri {
            var: addr_https,
            source: err,
        })?;
        let mut http = hyper::client::HttpConnector::new();
        http.enforce_http(false);
        let https = hyper_tls::HttpsConnector::from((http, builder.build()?.into()));
        let client = hyper::Client::builder().build::<_, hyper::Body>(https);
        Ok(Self::new(Client::HttpsClient(client), url))
    }

    #[cfg(feature = "rustls")]
    pub fn connect_with_ssl(
        addr: &str,
        key: &Path,
        cert: &Path,
        ca: &Path,
    ) -> Result<Self, DwError> {
        use log::warn;
        use rustls::{Certificate, PrivateKey};
        use rustls_pemfile::Item;
        use std::fs::File;
        use std::io::BufReader;

        let addr_https = addr.clone().replacen("tcp://", "https://", 1);
        let url = Uri::from_str(&addr_https).map_err(|err| DwError::InvalidUri {
            var: addr_https,
            source: err,
        })?;

        let mut key_buf = BufReader::new(File::open(key)?);
        let mut cert_buf = BufReader::new(File::open(cert)?);
        let mut ca_buf = BufReader::new(File::open(ca)?);

        let private_key = match rustls_pemfile::rsa_private_keys(&mut key_buf)? {
            keys if keys.is_empty() => return Err(rustls::Error::NoCertificatesPresented.into()),
            mut keys if keys.len() == 1 => PrivateKey(keys.remove(0)),
            mut keys => {
                // if keys.len() > 1
                warn!("Private key file contains multiple keys. Using only first one.");
                PrivateKey(keys.remove(0))
            }
        };
        let certs = rustls_pemfile::read_all(&mut cert_buf)?
            .into_iter()
            .filter_map(|item| match item {
                Item::X509Certificate(c) => Some(Certificate(c)),
                _ => None,
            })
            .collect();
        let mut root_certs = rustls::RootCertStore::empty();
        for c in rustls_pemfile::certs(&mut ca_buf)? {
            root_certs.add(&Certificate(c))?;
        }

        let config = rustls::ClientConfig::builder()
            .with_safe_default_cipher_suites()
            .with_safe_default_kx_groups()
            .with_safe_default_protocol_versions()
            .unwrap()
            .with_root_certificates(root_certs)
            .with_single_cert(certs, private_key)
            .expect("bad certificate/key");
        let https = hyper_rustls::HttpsConnectorBuilder::new()
            .with_tls_config(config)
            .https_or_http()
            .enable_all_versions()
            .build();
        let client = hyper::Client::builder().build::<_, hyper::Body>(https);
        Ok(Self::new(Client::HttpsClient(client), url))
    }

    pub fn connect_with_http(addr: &str) -> Result<Self, DwError> {
        // This ensures that using docker-machine-esque addresses work with Hyper.
        let addr_https = addr.to_string().replace("tcp://", "http://");
        let url = Uri::from_str(&addr_https).map_err(|err| DwError::InvalidUri {
            var: addr_https,
            source: err,
        })?;
        Ok(Self::new(Client::HttpClient(hyper::Client::new()), url))
    }
}

#[async_trait::async_trait]
impl HttpClient for HyperClient {
    type Err = DwError;

    async fn get(&self, headers: &HeaderMap, path: &str) -> Result<Response<Vec<u8>>, Self::Err> {
        let url = join_uri(&self.base, path)?;

        let res = request_with_redirect::<Vec<u8>>(
            self.client.clone(),
            http::Method::GET,
            url,
            headers.clone(),
            None,
        )
        .await?;
        let res = fetch_body(res).await?;
        Ok(res)
    }
    async fn get_stream(
        &self,
        headers: &HeaderMap,
        path: &str,
    ) -> Result<Response<hyper::Body>, Self::Err> {
        let url = join_uri(&self.base, path)?;

        let res = request_with_redirect::<Vec<u8>>(
            self.client.clone(),
            http::Method::GET,
            url,
            headers.clone(),
            None,
        )
        .await?;
        Ok(res)
    }

    async fn head(&self, headers: &HeaderMap, path: &str) -> Result<HeaderMap, Self::Err> {
        let url = join_uri(&self.base, path)?;

        let res = request_with_redirect::<Vec<u8>>(
            self.client.clone(),
            http::Method::HEAD,
            url,
            headers.clone(),
            None,
        )
        .await?;

        Ok(res.headers().clone())
    }

    async fn post(
        &self,
        headers: &HeaderMap,
        path: &str,
        body: &str,
    ) -> Result<Response<Vec<u8>>, Self::Err> {
        let url = join_uri(&self.base, path)?;

        let res = request_with_redirect(
            self.client.clone(),
            http::Method::POST,
            url,
            headers.clone(),
            Some(body.to_string()),
        )
        .await?;
        let res = fetch_body(res).await?;
        Ok(res)
    }

    async fn post_stream(
        &self,
        headers: &HeaderMap,
        path: &str,
        body: &str,
    ) -> Result<Response<hyper::Body>, Self::Err> {
        let url = join_uri(&self.base, path)?;

        let res = request_with_redirect(
            self.client.clone(),
            http::Method::POST,
            url,
            headers.clone(),
            Some(body.to_string()),
        )
        .await?;
        Ok(res)
    }

    async fn post_file(
        &self,
        headers: &HeaderMap,
        path: &str,
        file: &Path,
    ) -> Result<Response<Vec<u8>>, Self::Err> {
        let mut content = tokio::fs::File::open(file).await?;
        let url = join_uri(&self.base, path)?;

        use tokio::io::AsyncReadExt;
        let mut buf = Vec::new();
        content.read_to_end(&mut buf).await?;

        let res = request_with_redirect(
            self.client.clone(),
            http::Method::POST,
            url,
            headers.clone(),
            Some(buf),
        )
        .await?;
        let res = fetch_body(res).await?;
        Ok(res)
    }

    async fn post_file_stream(
        &self,
        headers: &HeaderMap,
        path: &str,
        file: &Path,
    ) -> Result<Response<hyper::Body>, Self::Err> {
        let mut content = tokio::fs::File::open(file).await?;
        let url = join_uri(&self.base, path)?;

        use tokio::io::AsyncReadExt;
        let mut buf = Vec::new();
        content.read_to_end(&mut buf).await?;

        let res = request_with_redirect(
            self.client.clone(),
            http::Method::POST,
            url,
            headers.clone(),
            Some(buf),
        )
        .await?;
        Ok(res)
    }

    async fn delete(
        &self,
        headers: &HeaderMap,
        path: &str,
    ) -> Result<Response<Vec<u8>>, Self::Err> {
        let url = join_uri(&self.base, path)?;

        let res = request_with_redirect::<Vec<u8>>(
            self.client.clone(),
            http::Method::DELETE,
            url,
            headers.clone(),
            None,
        )
        .await?;
        let res = fetch_body(res).await?;
        Ok(res)
    }

    async fn put_file(
        &self,
        headers: &HeaderMap,
        path: &str,
        file: &Path,
    ) -> Result<Response<Vec<u8>>, Self::Err> {
        let mut content = tokio::fs::File::open(file).await?;
        let url = join_uri(&self.base, path)?;

        use tokio::io::AsyncReadExt;
        let mut buf = Vec::new();
        content.read_to_end(&mut buf).await?;

        let res = request_with_redirect(
            self.client.clone(),
            http::Method::PUT,
            url,
            headers.clone(),
            Some(buf),
        )
        .await?;
        let res = fetch_body(res).await?;
        Ok(res)
    }
}
