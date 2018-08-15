use std::result;
use std::fmt;
use std::env;
use std::path::{Path, PathBuf};
use std::io::{BufRead, BufReader, Read};
use url;
use hyper::header::{ContentType, Headers};
use hyper::mime::{Mime, SubLevel, TopLevel};
use hyper::client::IntoUrl;
use hyper::client::response::Response;
use hyper::status::StatusCode;

use errors::*;
use container::{Container, ContainerInfo};
use options::*;
use process::{Process, Top};
use stats::StatsReader;
use system::SystemInfo;
use image::Image;
use filesystem::FilesystemChange;
use version::Version;
use hyper_client::HyperClient;

use serde::de::DeserializeOwned;
use serde_json::{self, Value};

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
    let from_env = env::var("DOCKER_CERT_PATH").or_else(|_| env::var("DOCKER_CONFIG"));
    if let Ok(ref path) = from_env {
        Ok(Path::new(path).to_owned())
    } else {
        let home = env::home_dir().ok_or_else(|| ErrorKind::NoCertPath)?;
        Ok(home.join(".docker"))
    }
}

/// protocol connect to docker daemon
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
enum Protocol {
    Unix,
    Tcp,
}

/// Client connects to docker daemon
#[derive(Debug)]
pub struct Docker {
    /// http client
    client: HyperClient,
    /// connection protocol
    protocol: Protocol,
    headers: Headers,
}

/// Type of general docker error response
#[derive(Debug, Deserialize)]
pub struct DockerError {
    pub message: String,
}

impl fmt::Display for DockerError {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(fmt, "{}", self.message)
    }
}

impl ::std::error::Error for DockerError {
    fn description(&self) -> &str {
        &self.message
    }

    fn cause(&self) -> Option<&::std::error::Error> {
        None
    }
}

/// Deserialize from json string
fn api_result<D: DeserializeOwned>(res: Response) -> result::Result<D, Error> {
    if res.status.is_success() {
        Ok(serde_json::from_reader::<_, D>(res)?)
    } else {
        Err(serde_json::from_reader::<_, DockerError>(res)?.into())
    }
}

/// Expect 204 NoContent
fn no_content(res: Response) -> result::Result<(), Error> {
    if res.status == StatusCode::NoContent {
        Ok(())
    } else {
        Err(serde_json::from_reader::<_, DockerError>(res)?.into())
    }
}

/// Ignore succeed response
fn ignore_result(res: Response) -> result::Result<(), Error> {
    if res.status.is_success() {
        Ok(()) // ignore
    } else {
        Err(serde_json::from_reader::<_, DockerError>(res)?.into())
    }
}

pub trait HttpClient {
    type Err: ::std::error::Error + Send + 'static;

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
}

pub trait HaveHttpClient {
    type Client: HttpClient;
    fn http_client(&self) -> &Self::Client;
}

impl Docker {
    fn new(client: HyperClient, protocol: Protocol) -> Self {
        Self {
            client,
            protocol,
            headers: Headers::new(),
        }
    }

    fn headers(&self) -> &Headers {
        &self.headers
    }

    /// Connect to the Docker daemon using the standard Docker
    /// configuration options.  This includes `DOCKER_HOST`,
    /// `DOCKER_TLS_VERIFY`, `DOCKER_CERT_PATH` and `DOCKER_CONFIG`, and we
    /// try to interpret these as much like the standard `docker` client as
    /// possible.
    pub fn connect_with_defaults() -> Result<Docker> {
        // Read in our configuration from the Docker environment.
        let host = env::var("DOCKER_HOST").unwrap_or(DEFAULT_DOCKER_HOST.to_string());
        let tls_verify = env::var("DOCKER_TLS_VERIFY").is_ok();
        let cert_path = default_cert_path()?;

        // Dispatch to the correct connection function.
        let mkerr = || ErrorKind::CouldNotConnect(host.clone());
        if host.starts_with("unix://") {
            Docker::connect_with_unix(&host).chain_err(&mkerr)
        } else if host.starts_with("tcp://") {
            if tls_verify {
                Docker::connect_with_ssl(
                    &host,
                    &cert_path.join("key.pem"),
                    &cert_path.join("cert.pem"),
                    &cert_path.join("ca.pem"),
                ).chain_err(&mkerr)
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
        let url = addr.into_url()?;
        let client = HyperClient::connect_with_unix(url.path());
        Ok(Docker::new(client, Protocol::Unix))
    }

    #[cfg(not(unix))]
    pub fn connect_with_unix(addr: &str) -> Result<Docker> {
        Err(ErrorKind::UnsupportedScheme(addr.to_owned()).into())
    }

    #[cfg(feature = "openssl")]
    pub fn connect_with_ssl(addr: &str, key: &Path, cert: &Path, ca: &Path) -> Result<Docker> {
        let client = HyperClient::connect_with_ssl(addr, key, cert, ca)?;
        Ok(Docker::new(client, Protocol::Tcp))
    }

    #[cfg(not(feature = "openssl"))]
    pub fn connect_with_ssl(_addr: &str, _key: &Path, _cert: &Path, _ca: &Path) -> Result<Docker> {
        Err(ErrorKind::SslDisabled.into())
    }

    /// Connect using unsecured HTTP.  This is strongly discouraged
    /// everywhere but on Windows when npipe support is not available.
    pub fn connect_with_http(addr: &str) -> Result<Docker> {
        let client = HyperClient::connect_with_http(addr)?;
        Ok(Docker::new(client, Protocol::Tcp))
    }

    /// List containers
    ///
    /// # API
    /// /containers/json
    pub fn containers(&self, opts: ContainerListOptions) -> Result<Vec<Container>> {
        self.http_client()
            .get(
                self.headers(),
                &format!("/containers/json?{}", opts.to_url_params()),
            )
            .and_then(api_result)
    }

    /// Create a container
    ///
    /// POST /containers/create
    pub fn create_container(
        &self,
        name: &str,
        option: &ContainerCreateOptions,
    ) -> Result<CreateContainerResponse> {
        let mut param = url::form_urlencoded::Serializer::new(String::new());
        param.append_pair("name", name);

        let json_body = serde_json::to_string(&option)?;
        let mut headers = self.headers().clone();
        headers.set::<ContentType>(ContentType::json());
        self.http_client()
            .post(
                &headers,
                &format!("/containers/create?{}", param.finish()),
                &json_body,
            )
            .and_then(api_result)
    }

    /// start a container
    ///
    /// # API
    /// /containers/{id}/start
    pub fn start_container(&self, id: &str) -> Result<()> {
        self.http_client()
            .post(self.headers(), &format!("/containers/{}/start", id), "")
            .and_then(no_content)
    }

    /// Attach to a container
    ///
    /// Attach to a container to read its output or send it input.
    ///
    /// # API
    /// /containers/{id}/attach
    #[allow(non_snake_case)]
    pub fn attach_container(
        &self,
        id: &str,
        detachKeys: Option<&str>,
        logs: bool,
        stream: bool,
        stdin: bool,
        stdout: bool,
        stderr: bool,
    ) -> Result<Response> {
        let mut param = url::form_urlencoded::Serializer::new(String::new());
        if let Some(keys) = detachKeys {
            param.append_pair("detachKeys", keys);
        }
        param.append_pair("logs", &logs.to_string());
        param.append_pair("stream", &stream.to_string());
        param.append_pair("stdin", &stdin.to_string());
        param.append_pair("stdout", &stdout.to_string());
        param.append_pair("stderr", &stderr.to_string());

        self.http_client().post(
            self.headers(),
            &format!("/containers/{}/attach?{}", id, param.finish()),
            "",
        )
    }

    /// List processes running inside a container
    ///
    /// # API
    /// /containers/{id}/top
    pub fn container_top(&self, container: &Container) -> Result<Top> {
        self.http_client()
            .get(self.headers(), &format!("/containers/{}/top", container.Id))
            .and_then(api_result)
    }

    pub fn processes(&self, container: &Container) -> Result<Vec<Process>> {
        let top = self.container_top(container)?;
        Ok(top.Processes
            .iter()
            .map(|process| {
                let mut p = Process::default();
                for (i, value) in process.iter().enumerate() {
                    let v = value.clone();
                    match top.Titles[i].as_ref() {
                        "UID" => p.user = v,
                        "USER" => p.user = v,
                        "PID" => p.pid = v,
                        "%CPU" => p.cpu = Some(v),
                        "%MEM" => p.memory = Some(v),
                        "VSZ" => p.vsz = Some(v),
                        "RSS" => p.rss = Some(v),
                        "TTY" => p.tty = Some(v),
                        "STAT" => p.stat = Some(v),
                        "START" => p.start = Some(v),
                        "STIME" => p.start = Some(v),
                        "TIME" => p.time = Some(v),
                        "CMD" => p.command = v,
                        "COMMAND" => p.command = v,
                        _ => {}
                    }
                }
                p
            })
            .collect())
    }

    /// Get containers stats based resource usage
    ///
    /// # API
    /// /containers/{id}/stats
    pub fn stats(&self, container: &Container) -> Result<StatsReader> {
        let res = self.http_client().get(
            self.headers(),
            &format!("/containers/{}/stats", container.Id),
        )?;
        Ok(StatsReader::new(res))
    }

    /// Create an image by pulling it from registry
    ///
    /// # API
    /// /images/create
    ///
    /// # TODO
    /// - Typing result iterator like image::ImageStatus.
    /// - Generalize input parameters
    pub fn create_image(
        &self,
        image: &str,
        tag: &str,
    ) -> Result<Box<Iterator<Item = Result<Value>>>> {
        let mut param = url::form_urlencoded::Serializer::new(String::new());
        param.append_pair("fromImage", image);
        param.append_pair("tag", tag);

        let res = self.http_client().post(
            self.headers(),
            &format!("/images/create?{}", param.finish()),
            "",
        )?;
        if res.status.is_success() {
            Ok(Box::new(BufReader::new(res).lines().map(|line| {
                Ok(line?).and_then(|ref line| Ok(serde_json::from_str(line)?))
            })))
        } else {
            Err(serde_json::from_reader::<_, DockerError>(res)?.into())
        }
    }

    /// Remove an image
    ///
    /// # API
    /// /images/{name}
    ///
    pub fn remove_image(
        &self,
        name: &str,
        force: Option<bool>,
        noprune: Option<bool>,
    ) -> Result<()> {
        let mut param = url::form_urlencoded::Serializer::new(String::new());
        param.append_pair("force", &force.unwrap_or(false).to_string());
        param.append_pair("noprune", &noprune.unwrap_or(false).to_string());
        self.http_client()
            .delete(
                self.headers(),
                &format!("/images/{}?{}", name, param.finish()),
            )
            .and_then(ignore_result)
    }

    /// List images
    ///
    /// # API
    /// /images/json
    pub fn images(&self, all: bool) -> Result<Vec<Image>> {
        self.http_client()
            .get(self.headers(), &format!("/images/json?a={}", all as u32))
            .and_then(api_result)
    }

    /// Load a set of images and tags
    ///
    /// # API
    /// /images/load
    pub fn load_image(&self, suppress: bool, path: &Path) -> Result<()> {
        let mut headers = self.headers().clone();
        let application_tar = Mime(TopLevel::Application, SubLevel::Ext("x-tar".into()), vec![]);
        headers.set::<ContentType>(ContentType(application_tar));
        self.http_client()
            .post_file(&headers, &format!("/images/load?quiet={}", suppress), path)
            .and_then(ignore_result)
    }

    /// Get system information
    ///
    /// # API
    /// /info
    pub fn system_info(&self) -> Result<SystemInfo> {
        self.http_client()
            .get(self.headers(), "/info")
            .and_then(api_result)
    }

    /// Inspect about a container
    ///
    /// # API
    /// /containers/{id}/json
    pub fn container_info(&self, container: &Container) -> Result<ContainerInfo> {
        self.http_client()
            .get(
                self.headers(),
                &format!("/containers/{}/json", container.Id),
            )
            .and_then(api_result)
    }

    /// Get changes on a container's filesystem
    ///
    /// # API
    /// /containers/{id}/changes
    pub fn filesystem_changes(&self, container: &Container) -> Result<Vec<FilesystemChange>> {
        self.http_client()
            .get(
                self.headers(),
                &format!("/containers/{}/changes", container.Id),
            )
            .and_then(api_result)
    }

    /// Export a container
    ///
    /// # Summary
    /// Returns a pointer to tar archive stream.
    ///
    /// # API
    /// /containers/{id}/export
    pub fn export_container(&self, container: &Container) -> Result<Box<Read>> {
        self.http_client()
            .get(
                self.headers(),
                &format!("/containers/{}/export", container.Id),
            )
            .and_then(|res| {
                if res.status.is_success() {
                    Ok(Box::new(res) as Box<Read>)
                } else {
                    Err(serde_json::from_reader::<_, DockerError>(res)?.into())
                }
            })
    }

    /// Test if the server is accessible
    ///
    /// # API
    /// /_ping
    pub fn ping(&self) -> Result<()> {
        let mut res = self.http_client().get(self.headers(), "/_ping")?;
        if res.status.is_success() {
            let mut buf = String::new();
            res.read_to_string(&mut buf)?;
            assert_eq!(&buf, "OK");
            Ok(())
        } else {
            Err(serde_json::from_reader::<_, DockerError>(res)?.into())
        }
    }

    /// Get version and various information
    ///
    /// # API
    /// /version
    pub fn version(&self) -> Result<Version> {
        self.http_client()
            .get(self.headers(), "/version")
            .and_then(api_result)
    }
}

impl HaveHttpClient for Docker {
    type Client = HyperClient;
    fn http_client(&self) -> &Self::Client {
        &self.client
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_remove_image() {
        let docker = Docker::connect_with_defaults().unwrap();
        let name = "debian";
        let tag = "latest";
        assert!(docker.create_image(name, tag).is_ok());
        assert!(
            docker
                .remove_image(&format!("{}:{}", name, tag), None, None)
                .is_ok()
        )
    }
}
