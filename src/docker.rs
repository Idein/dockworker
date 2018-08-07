use std;
use std::fs::File;
use std::collections::BTreeMap;
use std::env;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::io::{self, Read};
use url;
use hyper;
use hyper::header::ContentType;
use hyper::mime::*;
use hyper::Url;
use hyper::mime::{Mime, SubLevel, TopLevel};
use hyper::Client;
use hyper::client::{IntoUrl, RequestBuilder};
use hyper::client::pool::{Config, Pool};
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
#[cfg(unix)]
use unix::HttpUnixConnector;

use errors::*;
use container::{Container, ContainerInfo};
use options::*;
use process::{Process, Top};
use stats::StatsReader;
use system::SystemInfo;
use image::{Image, ImageStatus};
use filesystem::FilesystemChange;
use version::Version;

use serde::de::DeserializeOwned;
use serde_json;

/// The default `DOCKER_HOST` address that we will try to connect to.
#[cfg(unix)]
pub const DEFAULT_DOCKER_HOST: &'static str = "unix:///var/run/docker.sock";

/// The default `DOCKER_HOST` address that we will try to connect to.
///
/// This should technically be `"npipe:////./pipe/docker_engine"` on
/// Windows, but we don't support Windows pipes yet.  However, the TCP port
/// is still available.
#[cfg(windows)]
pub const DEFAULT_DOCKER_HOST: &'static str = "tcp://localhost:2375";

/// The default directory in which to look for our Docker certificate
/// files.
pub fn default_cert_path() -> Result<PathBuf> {
    let from_env = env::var("DOCKER_CERT_PATH")
        .or_else(|_| env::var("DOCKER_CONFIG"));
    if let Ok(ref path) = from_env {
        Ok(Path::new(path).to_owned())
    } else {
        let home = env::home_dir()
            .ok_or_else(|| ErrorKind::NoCertPath)?;
        Ok(home.join(".docker"))
    }
}

enum ClientType {
    Unix,
    Tcp,
}

pub struct Docker {
    client: Client,
    client_type: ClientType,
    client_addr: Url,
}

impl Docker {
    /// Connect to the Docker daemon using the standard Docker
    /// configuration options.  This includes `DOCKER_HOST`,
    /// `DOCKER_TLS_VERIFY`, `DOCKER_CERT_PATH` and `DOCKER_CONFIG`, and we
    /// try to interpret these as much like the standard `docker` client as
    /// possible.
    pub fn connect_with_defaults() -> Result<Docker> {
        // Read in our configuration from the Docker environment.
        let host = env::var("DOCKER_HOST")
            .unwrap_or(DEFAULT_DOCKER_HOST.to_string());
        let tls_verify = env::var("DOCKER_TLS_VERIFY").is_ok();
        let cert_path = default_cert_path()?;

        // Dispatch to the correct connection function.
        let mkerr = || ErrorKind::CouldNotConnect(host.clone());
        if host.starts_with("unix://") {
            Docker::connect_with_unix(&host).chain_err(&mkerr)
        } else if host.starts_with("tcp://") {
            if tls_verify {
                Docker::connect_with_ssl(&host,
                                         &cert_path.join("key.pem"),
                                         &cert_path.join("cert.pem"),
                                         &cert_path.join("ca.pem"))
                    .chain_err(&mkerr)
            } else {
                Docker::connect_with_http(&host).chain_err(&mkerr)
            }
        } else {
            Err(ErrorKind::UnsupportedScheme(host.clone()).into())
        }
    }

    #[cfg(unix)]
    pub fn connect_with_unix(addr: &str) -> Result<Docker> {
        // This ensures that using a fully-qualified path --
        // e.g. unix://.... -- works.  The unix socket provider expects a
        // Path, so we don't need scheme.
        let client_addr = addr.into_url()?;

        let http_unix_connector = HttpUnixConnector::new(client_addr.path());
        let connection_pool_config = Config { max_idle: 8 };
        let connection_pool = Pool::with_connector(connection_pool_config, http_unix_connector);

        let client = Client::with_connector(connection_pool);
        let docker = Docker { client: client, client_type: ClientType::Unix, client_addr: client_addr };

        return Ok(docker);
    }

    #[cfg(not(unix))]
    pub fn connect_with_unix(addr: &str) -> Result<Docker> {
        Err(ErrorKind::UnsupportedScheme(addr.to_owned()).into())
    }

    #[cfg(feature="openssl")]
    pub fn connect_with_ssl(addr: &str, ssl_key: &Path, ssl_cert: &Path, ssl_ca: &Path) -> Result<Docker> {
        // This ensures that using docker-machine-esque addresses work with Hyper.
        let client_addr = Url::parse(&addr.clone().replace("tcp://", "https://"))?;

        let mkerr = || ErrorKind::SslError(addr.to_owned());
        let mut ssl_context = SslContext::new(SslMethod::Sslv23).chain_err(&mkerr)?;
        ssl_context.set_CA_file(ssl_ca).chain_err(&mkerr)?;
        ssl_context.set_certificate_file(ssl_cert, X509FileType::PEM).chain_err(&mkerr)?;
        ssl_context.set_private_key_file(ssl_key, X509FileType::PEM).chain_err(&mkerr)?;

        let hyper_ssl_context = Openssl { context: Arc::new(ssl_context) };
        let https_connector = HttpsConnector::new(hyper_ssl_context);
        let connection_pool_config = Config { max_idle: 8 };
        let connection_pool = Pool::with_connector(connection_pool_config, https_connector);

        let client = Client::with_connector(connection_pool);
        let docker = Docker {
            client: client,
            client_type: ClientType::Tcp,
            client_addr: client_addr,
        };

        return Ok(docker);
    }

    #[cfg(not(feature="openssl"))]
    pub fn connect_with_ssl(addr: &str, ssl_key: &Path, ssl_cert: &Path, ssl_ca: &Path) -> Result<Docker> {
        Err(ErrorKind::SslDisabled.into())
    }

    /// Connect using unsecured HTTP.  This is strongly discouraged
    /// everywhere but on Windows when npipe support is not available.
    pub fn connect_with_http(addr: &str) -> Result<Docker> {
        // This ensures that using docker-machine-esque addresses work with Hyper.
        let client_addr = Url::parse(&addr.clone().replace("tcp://", "http://"))?;

        let http_connector = HttpConnector::default();
        let connection_pool_config = Config { max_idle: 8 };
        let connection_pool = Pool::with_connector(connection_pool_config, http_connector);

        let client = Client::with_connector(connection_pool);
        let docker = Docker { client: client, client_type: ClientType::Tcp, client_addr: client_addr };

        return Ok(docker);

    }

    fn get_url(&self, path: &str) -> Result<Url> {
        let base = match self.client_type {
            ClientType::Tcp => self.client_addr.clone(),
            ClientType::Unix => {
                // We need a host so the HTTP headers can be generated, so we just spoof it and say
                // that we're talking to localhost.  The hostname doesn't matter one bit.
                "http://localhost".into_url().expect("valid url")
            }
        };
        Ok(base.join(path)?)
    }

    fn build_get_request(&self, request_url: &Url) -> RequestBuilder {
        self.client.get(request_url.clone())
    }

    fn build_post_request(&self, request_url: &Url) -> RequestBuilder {
        self.client.post(request_url.clone())
    }

    fn execute_request(&self, request: RequestBuilder) -> Result<String> {
        let mut response = request.send()?;
        println!("{}", response.status);
        // assert!(response.status.is_success());

        let mut body = String::new();
        response.read_to_string(&mut body)?;
        Ok(body)
    }

    fn start_request(&self, request: RequestBuilder) -> Result<Response> {
        let response = request.send()?;
        assert!(response.status.is_success());
        Ok(response)
    }

    fn arrayify(&self, s: &str) -> String {
        let wrapped = format!("[{}]", s);
        wrapped.clone().replace("}\r\n{", "}{").replace("}{", "},{")
    }

    /// `GET` a URL and decode it.
    fn decode_url<T>(&self, type_name: &'static str, url: &str) -> Result<T>
        where T: DeserializeOwned<>
    {
        let request_url = self.get_url(url)?;
        let request = self.build_get_request(&request_url);
        let body = self.execute_request(request)?;
        let info = serde_json::from_str::<T>(&body)
            .chain_err(|| ErrorKind::ParseError(type_name, body))?;
        Ok(info)
    }

    pub fn containers(&self, opts: ContainerListOptions)
                      -> Result<Vec<Container>> {
        let url = format!("/containers/json?{}", opts.to_url_params());
        self.decode_url("Container", &url)
    }

    /// Create a container
    ///
    /// POST /containers/create
    pub fn create_container(&self, name: &str, create: &ContainerCreateOptions)
                    -> Result<CreateContainerResponse> {
        let mut name_param = url::form_urlencoded::Serializer::new(String::new());
        name_param.append_pair("name", name);

        let request_url = self.get_url(&format!("/containers/create?{}", name_param.finish()))?;
        let json_body = serde_json::to_string(&create)?;
        let request = self.build_post_request(&request_url)
                            .header(ContentType::json())
                            .body(&json_body);
        let response = self.execute_request(request)?;
        let container = serde_json::from_str(&response)
            .chain_err(|| ErrorKind::ParseError("CreateContainer", response))?;
        Ok(container)
    }

    /// start a container
    ///
    /// # API
    /// /containers/{id}/start
    pub fn start_container(&self, id: &str) -> Result<()> {
        let request_url = self.get_url(&format!("/containers/{}/start", id))?;
        let request = self.build_post_request(&request_url);
        let _response = self.execute_request(request)?;
        Ok(())
    }

    /// Attach to a container
    ///
    /// Attach to a container to read its output or send it input.
    ///
    /// # API
    /// /containers/{id}/attach
    pub fn attach_container(&self, id: &str, detachKeys: Option<&str>, logs: bool
                            , stream: bool, stdin: bool, stdout: bool, stderr: bool)
        -> Result<Response> {
        let mut param = url::form_urlencoded::Serializer::new(String::new());
        if let Some(keys) = detachKeys {
            param.append_pair("detachKeys", keys);
        }
        param.append_pair("logs", &logs.to_string());
        param.append_pair("stream", &stream.to_string());
        param.append_pair("stdin", &stdin.to_string());
        param.append_pair("stdout", &stdout.to_string());
        param.append_pair("stderr", &stderr.to_string());

        let request_url = self.get_url(&format!("/containers/{}/attach?{}", id, param.finish()))?;
        let request = self.build_post_request(&request_url);
        Ok(request.send()?)
    }

    pub fn processes(&self, container: &Container) -> Result<Vec<Process>> {
        let url = format!("/containers/{}/top", container.Id);
        let top: Top = self.decode_url("Top", &url)?;

        let mut processes: Vec<Process> = Vec::new();
        let mut process_iter = top.Processes.iter();
        loop {
            let process = match process_iter.next() {
                Some(process) => process,
                None => { break; }
            };

            let mut p = Process{
                user: String::new(),
                pid: String::new(),
                cpu: None,
                memory: None,
                vsz: None,
                rss: None,
                tty: None,
                stat: None,
                start: None,
                time: None,
                command: String::new()
            };

            let mut value_iter = process.iter();
            let mut i: usize = 0;
            loop {
                let value = match value_iter.next() {
                    Some(value) => value,
                    None => { break; }
                };
                let key = &top.Titles[i];
                match key.as_ref() {
                    "UID" => { p.user = value.clone() },
                    "USER" => {p.user = value.clone() },
                    "PID" => { p.pid = value.clone() },
                    "%CPU" => { p.cpu = Some(value.clone()) },
                    "%MEM" => { p.memory = Some(value.clone()) },
                    "VSZ" => { p.vsz = Some(value.clone()) },
                    "RSS" => { p.rss = Some(value.clone()) },
                    "TTY" => { p.tty = Some(value.clone()) },
                    "STAT" => { p.stat = Some(value.clone()) },
                    "START" => { p.start = Some(value.clone()) },
                    "STIME" => { p.start = Some(value.clone()) },
                    "TIME" => { p.time = Some(value.clone()) },
                    "CMD" => { p.command = value.clone() },
                    "COMMAND" => { p.command = value.clone() },
                    _ => {}
                }

                i = i + 1;
            }
            processes.push(p);
        }

        Ok(processes)
    }

    pub fn stats(&self, container: &Container) -> Result<StatsReader> {
        if container.Status.contains("Up") == false {
            return Err("The container is already stopped.".into());
        }

        let request_url = self.get_url(&format!("/containers/{}/stats", container.Id))?;
        let request = self.build_get_request(&request_url);
        let response = self.start_request(request)?;
        Ok(StatsReader::new(response))
    }

    pub fn create_image(&self, image: String, tag: String) -> Result<Vec<ImageStatus>> {
        let request_url = self.get_url(&format!("/images/create?fromImage={}&tag={}", image, tag))?;
        let request = self.build_post_request(&request_url);
        let body = self.execute_request(request)?;
        let fixed = self.arrayify(&body);
        let statuses = serde_json::from_str::<Vec<ImageStatus>>(&fixed)
            .chain_err(|| ErrorKind::ParseError("ImageStatus", fixed))?;
        Ok(statuses)
    }

    pub fn images(&self, all: bool) -> Result<Vec<Image>> {
        let a = match all {
            true => "1",
            false => "0"
        };
        let url = format!("/images/json?a={}", a);
        self.decode_url("Image", &url)
    }

    pub fn load_image(&self, suppress: bool, path: &Path) -> Result<()> {
        let mut file: File = File::open(path)?;
        let request_url = self.get_url(&format!("/images/load?quiet={}", suppress))?;
        let request = self.build_post_request(&request_url)
            .header(ContentType(Mime(
                TopLevel::Application,
                SubLevel::Ext("x-tar".into()),
                vec![],
            )))
            .body(&mut file);
        self.start_request(request)?;
        Ok(())
    }

    pub fn system_info(&self) -> Result<SystemInfo> {
        self.decode_url("SystemInfo", &format!("/info"))
    }

    pub fn container_info(&self, container: &Container) -> Result<ContainerInfo> {
        let url = format!("/containers/{}/json", container.Id);
        self.decode_url("ContainerInfo", &url)
            .chain_err(|| ErrorKind::ContainerInfo(container.Id.clone()))
    }

    pub fn filesystem_changes(&self, container: &Container) -> Result<Vec<FilesystemChange>> {
        let url = format!("/containers/{}/changes", container.Id);
        self.decode_url("FilesystemChange", &url)
    }

    pub fn export_container(&self, container: &Container) -> Result<Response> {
        let url = format!("/containers/{}/export", container.Id);
        let request_url = self.get_url(&url)?;
        let request = self.build_get_request(&request_url);
        Ok(self.start_request(request)?)
    }

    pub fn ping(&self) -> Result<String> {
        let request_url = self.get_url("/_ping")?;
        let request = self.build_get_request(&request_url);
        Ok(self.execute_request(request)?)
    }

    pub fn version(&self) -> Result<Version> {
        self.decode_url("Version", "/version")
    }
}
