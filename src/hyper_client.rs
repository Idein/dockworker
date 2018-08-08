use std::result;
use std::sync::Arc;
use std::path::Path;
use std::fs::File;

use hyper::client::{IntoUrl, RequestBuilder};
use hyper::Client;
use hyper::Url;
use hyper::client::pool::{Config, Pool};
use hyper::header::Headers;
use hyper::client::response::Response;
use hyper::net::HttpConnector;
#[cfg(feature="openssl")]
use hyper::net::{HttpsConnector, Openssl};
#[cfg(feature="openssl")]
use openssl::ssl::{SslContext, SslMethod};
#[cfg(feature="openssl")]
use openssl::ssl::error::SslError;
#[cfg(feature="openssl")]
use openssl::x509::X509FileType;

use unix::HttpUnixConnector;
use docker::*;
use errors::*;

/// Http client using hyper
#[derive(Debug)]
pub struct HyperClient {
    /// http client
    client: Client,
    /// base connection address
    base: Url,
}

fn ssl_context(addr: &str, key: &Path, cert: &Path, ca: &Path) -> result::Result<Openssl, Error> {
    let mkerr = || ErrorKind::SslError(addr.to_owned());
    let mut context = SslContext::new(SslMethod::Sslv23).chain_err(&mkerr)?;
    context.set_CA_file(ca).chain_err(&mkerr)?;
    context.set_certificate_file(cert, X509FileType::PEM).chain_err(&mkerr)?;
    context.set_private_key_file(key, X509FileType::PEM).chain_err(&mkerr)?;
    Ok(Openssl { context: Arc::new(context) })
}

impl HyperClient {
    fn new (client: Client, base: Url) -> Self {
        Self { client, base }
    }

    /// path to unix socket
    pub fn connect_with_unix(path: &str) -> Self {
        let conn = HttpUnixConnector::new(path);
        let pool_config = Config { max_idle: 8 };
        let pool = Pool::with_connector(pool_config, conn);

        // dummy base address
        let base_addr = "http://localhost".into_url().expect("dummy base url");
        let client = Client::with_connector(pool);
        Self::new(client, base_addr)
    }

    #[cfg(feature="openssl")]
    pub fn connect_with_ssl(addr: &str, key: &Path, cert: &Path, ca: &Path) -> result::Result<Self, Error> {
        // This ensures that using docker-machine-esque addresses work with Hyper.
        let url = Url::parse(&addr.clone().replacen("tcp://", "https://", 1))?;

        let ctx = ssl_context(addr, key, cert, ca)?;
        let conn = HttpsConnector::new(ctx);
        let pool_config = Config { max_idle: 8 };
        let pool = Pool::with_connector(pool_config, conn);

        let client = Client::with_connector(pool);
        Ok(Self::new(client, url))
    }

    pub fn connect_with_http(addr: &str) -> result::Result<Self, Error> {
        // This ensures that using docker-machine-esque addresses work with Hyper.
        let url = Url::parse(&addr.clone().replace("tcp://", "http://"))?;

        let conn = HttpConnector::default();
        let pool_config = Config { max_idle: 8 };
        let pool = Pool::with_connector(pool_config, conn);

        let client = Client::with_connector(pool);
        Ok(Self::new(client, url))
    }
}

impl HttpClient for HyperClient {
    type Err = Error;

    fn get(&self, headers: &Headers, path: &str) -> result::Result<Response, Self::Err> {
        let url = self.base.join(path)?;
        let res = self.client.get(url.clone()).headers(headers.clone()).send()?;
        Ok(res)
    }

    fn post(
        &self,
        headers: &Headers,
        path: &str,
        body: &str,
    ) -> result::Result<Response, Self::Err> {
        let url = self.base.join(path)?;
        let res = self.client
                .post(url.clone())
                .headers(headers.clone())
                .body(body)
                .send()?;
        Ok(res)
    }

    fn post_file(
        &self,
        headers: &Headers,
        path: &str,
        file: &Path,
    ) -> result::Result<Response, Self::Err> {
        let mut content = File::open(file)?;
        let url = self.base.join(path)?;
        let res = self.client
                .post(url.clone())
                .headers(headers.clone())
                .body(&mut content)
                .send()?;
        Ok(res)
    }
}

