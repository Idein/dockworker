#![allow(clippy::bool_assert_comparison)]
use crate::container::{
    AttachResponse, Container, ContainerFilters, ContainerInfo, ExecInfo, ExitStatus, LogResponse,
};
pub use crate::credentials::{Credential, UserPassword};
use crate::errors::*;
use crate::event::EventResponse;
use crate::filesystem::{FilesystemChange, XDockerContainerPathStat};
use crate::http_client::{HaveHttpClient, HttpClient};
use crate::hyper_client::{HyperClient, Response};
use crate::image::{Image, ImageId, SummaryImage};
use crate::network::*;
use crate::options::*;
use crate::process::{Process, Top};
use crate::response::Response as DockerResponse;
use crate::signal::Signal;
use crate::stats::StatsReader;
use crate::system::{AuthToken, SystemInfo};
use crate::version::Version;
#[cfg(feature = "experimental")]
use checkpoint::{Checkpoint, CheckpointCreateOptions, CheckpointDeleteOptions};
use http::{HeaderMap, StatusCode};
use log::*;
use serde::de::DeserializeOwned;
use serde::Deserialize;
use std::env;
use std::ffi::OsStr;
use std::fmt;
use std::fs::File;
use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::result;
use std::time::Duration;
use tar::Archive;

/// The default `DOCKER_HOST` address that we will try to connect to.
#[cfg(unix)]
pub static DEFAULT_DOCKER_HOST: &str = "unix:///var/run/docker.sock";

/// The default `DOCKER_HOST` address that we will try to connect to.
///
/// This should technically be `"npipe:////./pipe/docker_engine"` on
/// Windows, but we don't support Windows pipes yet.  However, the TCP port
/// is still available.
#[cfg(windows)]
pub static DEFAULT_DOCKER_HOST: &'static str = "tcp://localhost:2375";

/// The default directory in which to look for our Docker certificate
/// files.
pub fn default_cert_path() -> Result<PathBuf> {
    let from_env = env::var("DOCKER_CERT_PATH").or_else(|_| env::var("DOCKER_CONFIG"));
    if let Ok(ref path) = from_env {
        Ok(PathBuf::from(path))
    } else {
        let home = dirs::home_dir().ok_or(Error::NoCertPath)?;
        Ok(home.join(".docker"))
    }
}

/// protocol connect to docker daemon
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
enum Protocol {
    /// unix domain socket
    Unix,
    /// tcp/ip (BSD like socket)
    Tcp,
}

/// Handle to connection to the docker daemon
#[derive(Debug)]
pub struct Docker {
    /// http client
    client: HyperClient,
    /// connection protocol
    #[allow(dead_code)]
    protocol: Protocol,
    /// http headers used for any requests
    headers: HeaderMap,
    /// access credential for accessing apis
    credential: Option<Credential>,
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

    fn cause(&self) -> Option<&(dyn std::error::Error + 'static)> {
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
    if res.status == StatusCode::NO_CONTENT {
        Ok(())
    } else {
        Err(serde_json::from_reader::<_, DockerError>(res)?.into())
    }
}

/// Expect 204 NoContent or 304 NotModified
fn no_content_or_not_modified(res: Response) -> result::Result<(), Error> {
    if res.status == StatusCode::NO_CONTENT || res.status == StatusCode::NOT_MODIFIED {
        Ok(())
    } else {
        Err(serde_json::from_reader::<_, DockerError>(res)?.into())
    }
}

/// Ignore succeed response
///
/// Read whole response body, then ignore it.
fn ignore_result(res: Response) -> result::Result<(), Error> {
    if res.status.is_success() {
        res.bytes().last(); // ignore
        Ok(())
    } else {
        Err(serde_json::from_reader::<_, DockerError>(res)?.into())
    }
}

impl Docker {
    fn new(client: HyperClient, protocol: Protocol) -> Self {
        Self {
            client,
            protocol,
            headers: HeaderMap::new(),
            credential: None,
        }
    }

    pub fn set_credential(&mut self, credential: Credential) {
        self.credential = Some(credential)
    }

    fn headers(&self) -> &HeaderMap {
        &self.headers
    }

    /// Connect to the Docker daemon
    ///
    /// # Summary
    /// Connect to the Docker daemon using the standard Docker configuration options.
    /// This includes:
    /// - `DOCKER_HOST`
    /// - `DOCKER_TLS_VERIFY`
    /// - `DOCKER_CERT_PATH`
    /// - `DOCKER_CONFIG`
    ///
    /// and we try to interpret these as much like the standard `docker` client as possible.
    pub fn connect_with_defaults() -> Result<Docker> {
        // Read in our configuration from the Docker environment.
        let host = env::var("DOCKER_HOST").unwrap_or_else(|_| DEFAULT_DOCKER_HOST.to_string());
        let tls_verify = env::var("DOCKER_TLS_VERIFY").is_ok();
        let cert_path = default_cert_path()?;

        // Dispatch to the correct connection function.
        if host.starts_with("unix://") {
            Docker::connect_with_unix(&host)
        } else if host.starts_with("tcp://") {
            if tls_verify {
                Docker::connect_with_ssl(
                    &host,
                    &cert_path.join("key.pem"),
                    &cert_path.join("cert.pem"),
                    &cert_path.join("ca.pem"),
                )
            } else {
                Docker::connect_with_http(&host)
            }
        } else {
            Err(Error::UnsupportedScheme { host })
        }
    }

    /// This ensures that using a fully-qualified path
    ///
    /// e.g. unix://.... -- works.
    /// The unix socket provider expects a Path, so we don't need scheme.
    #[cfg(unix)]
    pub fn connect_with_unix(addr: &str) -> Result<Docker> {
        if let Some(addr) = addr.strip_prefix("unix://") {
            let client = HyperClient::connect_with_unix(addr);
            Ok(Docker::new(client, Protocol::Unix))
        } else {
            let client = HyperClient::connect_with_unix(addr);
            Ok(Docker::new(client, Protocol::Unix))
        }
    }

    #[cfg(not(unix))]
    pub fn connect_with_unix(addr: &str) -> Result<Docker> {
        Err(Error::UnsupportedScheme {
            host: addr.to_owned(),
        }
        .into())
    }

    #[cfg(feature = "openssl")]
    pub fn connect_with_ssl(addr: &str, key: &Path, cert: &Path, ca: &Path) -> Result<Docker> {
        let client = HyperClient::connect_with_ssl(addr, key, cert, ca).map_err(|err| {
            Error::CouldNotConnect {
                addr: addr.to_owned(),
                source: err.into(),
            }
        })?;
        Ok(Docker::new(client, Protocol::Tcp))
    }

    #[cfg(not(feature = "openssl"))]
    pub fn connect_with_ssl(_addr: &str, _key: &Path, _cert: &Path, _ca: &Path) -> Result<Docker> {
        Err(Error::SslDisabled)
    }

    /// Connect using unsecured HTTP.  This is strongly discouraged
    /// everywhere but on Windows when npipe support is not available.
    pub fn connect_with_http(addr: &str) -> Result<Docker> {
        let client =
            HyperClient::connect_with_http(addr).map_err(|err| Error::CouldNotConnect {
                addr: addr.to_owned(),
                source: err.into(),
            })?;
        Ok(Docker::new(client, Protocol::Tcp))
    }

    /// List containers
    ///
    /// # API
    /// /containers/json
    pub fn list_containers(
        &self,
        all: Option<bool>,
        limit: Option<u64>,
        size: Option<bool>,
        filters: ContainerFilters,
    ) -> Result<Vec<Container>> {
        let mut param = url::form_urlencoded::Serializer::new(String::new());
        param.append_pair("all", &(all.unwrap_or(false) as u64).to_string());
        if let Some(limit) = limit {
            param.append_pair("limit", &limit.to_string());
        }
        param.append_pair("size", &(size.unwrap_or(false) as u64).to_string());
        param.append_pair("filters", &serde_json::to_string(&filters).unwrap());
        debug!("filter: {}", serde_json::to_string(&filters).unwrap());

        self.http_client()
            .get(
                self.headers(),
                &format!("/containers/json?{}", param.finish()),
            )
            .and_then(api_result)
    }

    #[deprecated(note = "please use `list_containers` instead")]
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
    /// # Summary
    ///
    /// * `name` - None: auto naming
    /// * `option` - create options
    ///
    /// # API
    /// POST /containers/create?{name}
    pub fn create_container(
        &self,
        name: Option<&str>,
        option: &ContainerCreateOptions,
    ) -> Result<CreateContainerResponse> {
        let path = match name {
            Some(name) => {
                let mut param = url::form_urlencoded::Serializer::new(String::new());
                param.append_pair("name", name);
                format!("/containers/create?{}", param.finish())
            }
            None => "/containers/create".to_string(),
        };

        let json_body = serde_json::to_string(&option)?;
        let mut headers = self.headers().clone();
        headers.insert(
            http::header::CONTENT_TYPE,
            "application/json".parse().unwrap(),
        );
        self.http_client()
            .post(&headers, &path, &json_body)
            .and_then(api_result)
    }

    /// Start a container
    ///
    /// # API
    /// /containers/{id}/start
    pub fn start_container(&self, id: &str) -> Result<()> {
        self.http_client()
            .post(self.headers(), &format!("/containers/{id}/start"), "")
            .and_then(no_content)
    }

    /// Start a container from a checkpoint
    ///
    /// Using normal container start endpoint with preconfigured arguments
    ///
    /// # API
    /// /containers/{id}/start
    #[cfg(feature = "experimental")]
    pub fn resume_container_from_checkpoint(
        &self,
        id: &str,
        checkpoint_id: &str,
        checkpoint_dir: Option<&str>,
    ) -> Result<()> {
        let mut param = url::form_urlencoded::Serializer::new(String::new());
        param.append_pair("checkpoint", &checkpoint_id);
        if let Some(dir) = checkpoint_dir {
            param.append_pair("checkpoint-dir", &dir);
        }
        self.http_client()
            .post(
                self.headers(),
                &format!("/containers/{}/start?{}", id, param.finish()),
                "",
            )
            .and_then(no_content)
    }

    /// Stop a container
    ///
    /// # API
    /// /containers/{id}/stop
    pub fn stop_container(&self, id: &str, timeout: Duration) -> Result<()> {
        let mut param = url::form_urlencoded::Serializer::new(String::new());
        param.append_pair("t", &timeout.as_secs().to_string());
        self.http_client()
            .post(
                self.headers(),
                &format!("/containers/{}/stop?{}", id, param.finish()),
                "",
            )
            .and_then(no_content_or_not_modified)
    }

    /// Kill a container
    ///
    /// # API
    /// /containers/{id}/kill
    pub fn kill_container(&self, id: &str, signal: Signal) -> Result<()> {
        let mut param = url::form_urlencoded::Serializer::new(String::new());
        param.append_pair("signal", &signal.as_i32().to_string());
        self.http_client()
            .post(
                self.headers(),
                &format!("/containers/{}/kill?{}", id, param.finish()),
                "",
            )
            .and_then(no_content)
    }

    /// Restart a container
    ///
    /// # API
    /// /containers/{id}/restart
    pub fn restart_container(&self, id: &str, timeout: Duration) -> Result<()> {
        let mut param = url::form_urlencoded::Serializer::new(String::new());
        param.append_pair("t", &timeout.as_secs().to_string());
        self.http_client()
            .post(
                self.headers(),
                &format!("/containers/{}/restart?{}", id, param.finish()),
                "",
            )
            .and_then(no_content)
    }

    /// Attach to a container
    ///
    /// Attach to a container to read its output or send it input.
    ///
    /// # API
    /// /containers/{id}/attach
    #[allow(non_snake_case)]
    #[allow(clippy::too_many_arguments)]
    pub fn attach_container(
        &self,
        id: &str,
        detachKeys: Option<&str>,
        logs: bool,
        stream: bool,
        stdin: bool,
        stdout: bool,
        stderr: bool,
    ) -> Result<AttachResponse> {
        let mut param = url::form_urlencoded::Serializer::new(String::new());
        if let Some(keys) = detachKeys {
            param.append_pair("detachKeys", keys);
        }
        param.append_pair("logs", &logs.to_string());
        param.append_pair("stream", &stream.to_string());
        param.append_pair("stdin", &stdin.to_string());
        param.append_pair("stdout", &stdout.to_string());
        param.append_pair("stderr", &stderr.to_string());

        self.http_client()
            .post(
                self.headers(),
                &format!("/containers/{}/attach?{}", id, param.finish()),
                "",
            )
            .and_then(|res| {
                if res.status.is_success() {
                    Ok(AttachResponse::new(res))
                } else {
                    Err(serde_json::from_reader::<_, DockerError>(res)?.into())
                }
            })
    }

    /// List existing checkpoints from container
    ///
    /// Lists all snapshots made from the container in the specified directory.
    ///
    /// # API
    /// GET /containers/{id}/checkpoints
    #[cfg(feature = "experimental")]
    #[allow(non_snake_case)]
    pub fn list_container_checkpoints(
        &self,
        id: &str,
        dir: Option<String>,
    ) -> Result<Vec<Checkpoint>> {
        let mut headers = self.headers().clone();
        headers.set::<ContentType>(ContentType::json());

        let mut param = url::form_urlencoded::Serializer::new(String::new());
        if let Some(_dir) = dir {
            param.append_pair("dir", &_dir);
        }

        self.http_client()
            .get(
                &headers,
                &format!("/containers/{}/checkpoints?{}", id, param.finish()),
            )
            .and_then(api_result)
    }

    /// Create Checkpoint from current running container
    ///
    /// Create a snapshot of the container's current state.
    ///
    /// # API
    /// POST /containers/{id}/checkpoints
    #[cfg(feature = "experimental")]
    #[allow(non_snake_case)]
    pub fn checkpoint_container(&self, id: &str, option: &CheckpointCreateOptions) -> Result<()> {
        let json_body = serde_json::to_string(&option)?;
        let mut headers = self.headers().clone();
        headers.set::<ContentType>(ContentType::json());
        self.http_client()
            .post(
                &headers,
                &format!("/containers/{}/checkpoints", id),
                &json_body,
            )
            .and_then(|res| {
                if res.status.is_success() && res.status == StatusCode::CREATED {
                    Ok(())
                } else {
                    Err(serde_json::from_reader::<_, DockerError>(res)?.into())
                }
            })
    }

    /// Delete a checkpoint
    ///
    /// Delete a snapshot of a container specified by its name.
    ///
    /// # API
    /// DELETE /containers/{id}/checkpoints
    #[cfg(feature = "experimental")]
    #[allow(non_snake_case)]
    pub fn delete_checkpoint(&self, id: &str, option: &CheckpointDeleteOptions) -> Result<()> {
        let mut headers = self.headers().clone();
        headers.set::<ContentType>(ContentType::json());

        let mut param = url::form_urlencoded::Serializer::new(String::new());
        let options = option.clone();
        if let Some(checkpoint_dir) = options.checkpoint_dir {
            param.append_pair("dir", &checkpoint_dir);
        }
        self.http_client()
            .delete(
                &headers,
                &format!(
                    "/containers/{}/checkpoints/{}?{}",
                    id,
                    option.checkpoint_id,
                    param.finish()
                ),
            )
            .and_then(no_content)
    }

    /// Create Exec instance for a container
    ///
    /// Run a command inside a running container.
    ///
    /// # API
    /// /containers/{id}/exec
    #[allow(non_snake_case)]
    pub fn exec_container(
        &self,
        id: &str,
        option: &CreateExecOptions,
    ) -> Result<CreateExecResponse> {
        let json_body = serde_json::to_string(&option)?;
        let mut headers = self.headers().clone();
        headers.insert(
            http::header::CONTENT_TYPE,
            "application/json".parse().unwrap(),
        );
        self.http_client()
            .post(&headers, &format!("/containers/{id}/exec"), &json_body)
            .and_then(api_result)
    }

    /// Start an exec instance
    ///
    /// Starts a previously set up exec instance. If detach is true, this endpoint returns immediately after starting the command. Otherwise, it sets up an interactive session with the command.
    ///
    /// # API
    /// /exec/{id}/start
    #[allow(non_snake_case)]
    pub fn start_exec(&self, id: &str, option: &StartExecOptions) -> Result<AttachResponse> {
        let json_body = serde_json::to_string(&option)?;

        let mut headers = self.headers().clone();
        headers.insert(
            http::header::CONTENT_TYPE,
            "application/json".parse().unwrap(),
        );

        self.http_client()
            .post(&headers, &format!("/exec/{id}/start"), &json_body)
            .and_then(|res| {
                if res.status.is_success() {
                    Ok(AttachResponse::new(res))
                } else {
                    Err(serde_json::from_reader::<_, DockerError>(res)?.into())
                }
            })
    }

    /// Inspect an exec instance
    ///
    /// Return low-level information about an exec instance.
    ///
    /// # API
    /// /exec/{id}/json
    #[allow(non_snake_case)]
    pub fn exec_inspect(&self, id: &str) -> Result<ExecInfo> {
        self.http_client()
            .get(self.headers(), &format!("/exec/{id}/json"))
            .and_then(api_result)
    }

    /// Gets current logs and tails logs from a container
    ///
    /// # API
    /// /containers/{id}/logs
    pub fn log_container(&self, id: &str, option: &ContainerLogOptions) -> Result<LogResponse> {
        self.http_client()
            .get(
                self.headers(),
                &format!("/containers/{}/logs?{}", id, option.to_url_params()),
            )
            .and_then(|res| {
                if res.status.is_success() {
                    Ok(res.into())
                } else {
                    Err(serde_json::from_reader::<_, DockerError>(res)?.into())
                }
            })
    }

    /// List processes running inside a container
    ///
    /// # API
    /// /containers/{id}/top
    pub fn container_top(&self, container_id: &str) -> Result<Top> {
        self.http_client()
            .get(self.headers(), &format!("/containers/{container_id}/top"))
            .and_then(api_result)
    }

    pub fn processes(&self, container_id: &str) -> Result<Vec<Process>> {
        let top = self.container_top(container_id)?;
        Ok(top
            .Processes
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
    /// GET /containers/{id}/stats
    pub fn stats(
        &self,
        container_id: &str,
        stream: Option<bool>,
        oneshot: Option<bool>,
    ) -> Result<StatsReader> {
        let mut query = url::form_urlencoded::Serializer::new(String::new());
        query.append_pair("stream", &stream.unwrap_or(true).to_string());
        query.append_pair("one-shot", &oneshot.unwrap_or(false).to_string());
        let res = self.http_client().get(
            self.headers(),
            &format!("/containers/{}/stats?{}", container_id, query.finish()),
        )?;
        if res.status.is_success() {
            Ok(StatsReader::new(res))
        } else {
            Err(serde_json::from_reader::<_, DockerError>(res)?.into())
        }
    }

    /// Wait for a container
    ///
    /// # API
    /// /containers/{id}/wait
    pub fn wait_container(&self, id: &str) -> Result<ExitStatus> {
        self.http_client()
            .post(self.headers(), &format!("/containers/{id}/wait"), "")
            .and_then(api_result)
    }

    /// Remove a container
    ///
    /// # API
    /// /containers/{id}
    pub fn remove_container(
        &self,
        id: &str,
        volume: Option<bool>,
        force: Option<bool>,
        link: Option<bool>,
    ) -> Result<()> {
        let mut param = url::form_urlencoded::Serializer::new(String::new());
        param.append_pair("v", &volume.unwrap_or(false).to_string());
        param.append_pair("force", &force.unwrap_or(false).to_string());
        param.append_pair("link", &link.unwrap_or(false).to_string());
        self.http_client()
            .delete(
                self.headers(),
                &format!("/containers/{}?{}", id, param.finish()),
            )
            .and_then(no_content)
    }

    /// Get an archive of a filesystem resource in a container
    ///
    /// # API
    /// /containers/{id}/archive
    pub fn get_file(&self, id: &str, path: &Path) -> Result<tar::Archive<Box<dyn Read>>> {
        let mut param = url::form_urlencoded::Serializer::new(String::new());
        debug!("get_file({}, {})", id, path.display());
        param.append_pair("path", path.to_str().unwrap_or("")); // FIXME: cause an invalid path error
        self.http_client()
            .get(
                self.headers(),
                &format!("/containers/{}/archive?{}", id, param.finish()),
            )
            .and_then(|res| {
                if res.status.is_success() {
                    Ok(tar::Archive::new(Box::new(res) as Box<dyn Read>))
                } else {
                    Err(serde_json::from_reader::<_, DockerError>(res)?.into())
                }
            })
    }

    /// Get information about files in a container
    ///
    /// # API
    /// /containers/{id}/archive
    pub fn head_file(&self, id: &str, path: &Path) -> Result<XDockerContainerPathStat> {
        let mut param = url::form_urlencoded::Serializer::new(String::new());
        debug!("head_file({}, {})", id, path.display());
        param.append_pair("path", path.to_str().unwrap_or(""));
        self.http_client()
            .head(
                self.headers(),
                &format!("/containers/{}/archive?{}", id, param.finish()),
            )
            .and_then(|res| {
                let stat_base64: &str = res
                    .get("X-Docker-Container-Path-Stat")
                    .map(|h| h.to_str().unwrap_or(""))
                    .unwrap_or("");
                let bytes = base64::decode(stat_base64).map_err(|err| Error::ParseError {
                    input: String::from(stat_base64),
                    source: err,
                })?;
                let path_stat: XDockerContainerPathStat = serde_json::from_slice(&bytes)?;
                Ok(path_stat)
            })
    }

    /// Extract an archive of files or folders to a directory in a container
    ///
    /// # Summary
    /// Extract given src file into the container specified with id.
    /// The input file must be a tar archive with id(no compress), gzip, bzip2 or xz.
    ///
    /// * id  : container name or ID
    /// * src : path to a source *file*
    /// * dst : path to a *directory* in the container to extract the archive's contents into
    ///
    /// # API
    /// /containers/{id}/archive
    #[allow(non_snake_case)]
    pub fn put_file(
        &self,
        id: &str,
        src: &Path,
        dst: &Path,
        noOverwriteDirNonDir: bool,
    ) -> Result<()> {
        debug!(
            "put_file({}, {}, {}, {})",
            id,
            src.display(),
            dst.display(),
            noOverwriteDirNonDir
        );
        let mut param = url::form_urlencoded::Serializer::new(String::new());
        param.append_pair("path", &dst.to_string_lossy());
        param.append_pair("noOverwriteDirNonDir", &noOverwriteDirNonDir.to_string());
        self.http_client()
            .put_file(
                self.headers(),
                &format!("/containers/{}/archive?{}", id, param.finish()),
                src,
            )
            .and_then(ignore_result)
    }

    /// Build an image from a tar archive with a Dockerfile in it.
    ///
    /// # API
    /// /build?
    pub fn build_image(&self, options: ContainerBuildOptions, tar_path: &Path) -> Result<Response> {
        let mut headers = self.headers().clone();
        headers.insert(
            http::header::CONTENT_TYPE,
            "application/x-tar".parse().unwrap(),
        );
        let res = self.http_client().post_file(
            &headers,
            &format!("/build?{}", options.to_url_params()),
            tar_path,
        )?;
        if !res.status.is_success() {
            return Err(serde_json::from_reader::<_, DockerError>(res)?.into());
        }

        Ok(res)
    }

    /// Create an image by pulling it from registry
    ///
    /// # API
    /// /images/create?fromImage={image}&tag={tag}
    ///
    /// # NOTE
    /// When control returns from this function, creating job may not have been completed.
    /// For waiting the completion of the job, consuming response like
    /// `create_image("hello-world", "linux").map(|r| r.for_each(|_| ()));`.
    ///
    /// # TODO
    /// - Typing result iterator like image::ImageStatus.
    /// - Generalize input parameters
    pub fn create_image(
        &self,
        image: &str,
        tag: &str,
    ) -> Result<Box<dyn Iterator<Item = Result<DockerResponse>>>> {
        let mut param = url::form_urlencoded::Serializer::new(String::new());
        param.append_pair("fromImage", image);
        param.append_pair("tag", tag);

        let mut headers = self.headers().clone();
        if let Some(ref credential) = self.credential {
            headers.insert(
                "X-Registry-Auth",
                base64::encode_config(
                    serde_json::to_string(credential).unwrap().as_bytes(),
                    base64::STANDARD,
                )
                .parse()
                .unwrap(),
            );
        }
        let res =
            self.http_client()
                .post(&headers, &format!("/images/create?{}", param.finish()), "")?;
        if res.status.is_success() {
            Ok(Box::new(
                BufReader::new(res)
                    .lines()
                    .map(|line| Ok(serde_json::from_str(&line?)?)),
            ))
        } else {
            Err(serde_json::from_reader::<_, DockerError>(res)?.into())
        }
    }

    /// Inspect an image
    ///
    /// # API
    /// /images/{name}/json
    ///
    pub fn inspect_image(&self, name: &str) -> Result<Image> {
        self.http_client()
            .get(self.headers(), &format!("/images/{name}/json"))
            .and_then(api_result)
    }

    /// Push an image
    ///
    /// # NOTE
    /// For pushing an image to non default registry, add registry id to prefix of the image name like `<registry>/<image>` .
    /// But the name of the local cache image is `<image>:<tag>` .
    ///
    /// # API
    /// /images/{name}/push
    ///
    pub fn push_image(&self, name: &str, tag: &str) -> Result<()> {
        let mut param = url::form_urlencoded::Serializer::new(String::new());
        param.append_pair("tag", tag);
        let mut headers = self.headers().clone();
        if let Some(ref credential) = self.credential {
            headers.insert(
                "X-Registry-Auth",
                base64::encode_config(
                    serde_json::to_string(credential).unwrap().as_bytes(),
                    base64::STANDARD,
                )
                .parse()
                .unwrap(),
            );
        }
        self.http_client()
            .post(
                &headers,
                &format!("/images/{}/push?{}", name, param.finish()),
                "",
            )
            .and_then(ignore_result)
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
    ) -> Result<Vec<RemovedImage>> {
        let mut param = url::form_urlencoded::Serializer::new(String::new());
        param.append_pair("force", &force.unwrap_or(false).to_string());
        param.append_pair("noprune", &noprune.unwrap_or(false).to_string());
        self.http_client()
            .delete(
                self.headers(),
                &format!("/images/{}?{}", name, param.finish()),
            )
            .and_then(api_result)
    }

    /// Delete unused images
    ///
    /// # API
    /// /images/prune
    pub fn prune_image(&self, dangling: bool) -> Result<PrunedImages> {
        debug!("start pruning...dangling? {}", &dangling);
        let mut param = url::form_urlencoded::Serializer::new(String::new());
        param.append_pair(
            "filters",
            &format!(r#"{{ "dangling": {{ "{dangling}": true }} }}"#),
        );
        self.http_client()
            .post(
                self.headers(),
                &format!("/images/prune?{}", param.finish()),
                "",
            )
            .and_then(api_result)
    }

    /// History of an image
    ///
    /// # API
    /// /images/{name}/history
    ///
    pub fn history_image(&self, name: &str) -> Result<Vec<ImageLayer>> {
        self.http_client()
            .get(self.headers(), &format!("/images/{name}/history"))
            .and_then(api_result)
            .map(|mut hs: Vec<ImageLayer>| {
                hs.iter_mut().for_each(|change| {
                    if change.id.as_deref() == Some("<missing>") {
                        change.id = None;
                    }
                });
                hs
            })
    }

    /// List images
    ///
    /// # API
    /// /images/json
    pub fn images(&self, all: bool) -> Result<Vec<SummaryImage>> {
        self.http_client()
            .get(self.headers(), &format!("/images/json?a={}", all as u32))
            .and_then(api_result)
    }

    /// Get a tarball containing all images and metadata for a repository
    ///
    /// # API
    /// /images/{name}/get
    pub fn export_image(&self, name: &str) -> Result<Box<dyn Read>> {
        self.http_client()
            .get(self.headers(), &format!("/images/{name}/get"))
            .and_then(|res| {
                if res.status.is_success() {
                    Ok(Box::new(res) as Box<dyn Read>)
                } else {
                    Err(serde_json::from_reader::<_, DockerError>(res)?.into())
                }
            })
    }

    /// Import images
    ///
    /// # Summary
    /// Load a set of images and tags into a repository
    ///
    /// # API
    /// /images/load
    pub fn load_image(&self, quiet: bool, path: &Path) -> Result<ImageId> {
        let mut headers = self.headers().clone();
        headers.insert(
            http::header::CONTENT_TYPE,
            "application/x-tar".parse().unwrap(),
        );
        let res =
            self.http_client()
                .post_file(&headers, &format!("/images/load?quiet={quiet}"), path)?;
        if !res.status.is_success() {
            return Err(serde_json::from_reader::<_, DockerError>(res)?.into());
        }
        // read and discard to end of response
        for line in BufReader::new(res).lines() {
            let buf = line?;
            debug!("{}", buf);
        }

        let mut ar = Archive::new(File::open(path)?);
        for entry in ar.entries()?.filter_map(|e| e.ok()) {
            let path = entry.path()?;
            // looking for file name like XXXXXXXXXXXXXX.json
            if path.extension() == Some(OsStr::new("json")) && path != Path::new("manifest.json") {
                let stem = path.file_stem().unwrap(); // contains .json
                let id = stem.to_str().ok_or(Error::Unknown {
                    message: format!("convert to String: {stem:?}"),
                })?;
                return Ok(ImageId::new(id.to_string()));
            }
        }
        Err(Error::Unknown {
            message: "no expected file: XXXXXX.json".to_owned(),
        })
    }

    /// Check auth configuration
    ///
    /// # API
    /// /auth
    ///
    /// # NOTE
    /// In some cases, docker daemon returns an empty token with `200 Ok`.
    /// The empty token could not be used for authenticating users.
    pub fn auth(
        &self,
        username: &str,
        password: &str,
        email: &str,
        serveraddress: &str,
    ) -> Result<AuthToken> {
        let req = UserPassword::new(
            username.to_string(),
            password.to_string(),
            email.to_string(),
            serveraddress.to_string(),
        );
        let json_body = serde_json::to_string(&req)?;
        let mut headers = self.headers().clone();
        headers.insert(
            http::header::CONTENT_TYPE,
            "application/json".parse().unwrap(),
        );
        self.http_client()
            .post(&headers, "/auth", &json_body)
            .and_then(api_result)
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
    pub fn container_info(&self, container_id: &str) -> Result<ContainerInfo> {
        self.http_client()
            .get(self.headers(), &format!("/containers/{container_id}/json"))
            .and_then(api_result)
    }

    /// Get changes on a container's filesystem.
    ///
    /// (This is the same as `docker container diff` command.)
    ///
    /// # API
    /// /containers/{id}/changes
    pub fn filesystem_changes(&self, container_id: &str) -> Result<Vec<FilesystemChange>> {
        self.http_client()
            .get(
                self.headers(),
                &format!("/containers/{container_id}/changes"),
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
    pub fn export_container(&self, container_id: &str) -> Result<Box<dyn Read>> {
        self.http_client()
            .get(
                self.headers(),
                &format!("/containers/{container_id}/export"),
            )
            .and_then(|res| {
                if res.status.is_success() {
                    Ok(Box::new(res) as Box<dyn Read>)
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

    /// Get monitor events
    ///
    /// # API
    /// /events
    pub fn events(
        &self,
        since: Option<u64>,
        until: Option<u64>,
        filters: Option<EventFilters>,
    ) -> Result<Box<dyn Iterator<Item = Result<EventResponse>>>> {
        let mut param = url::form_urlencoded::Serializer::new(String::new());

        if let Some(since) = since {
            param.append_pair("since", &since.to_string());
        }

        if let Some(until) = until {
            param.append_pair("until", &until.to_string());
        }

        if let Some(filters) = filters {
            param.append_pair("filters", &serde_json::to_string(&filters).unwrap());
        }

        self.http_client()
            .get(self.headers(), &format!("/events?{}", param.finish()))
            .map(|res| {
                Box::new(
                    serde_json::Deserializer::from_reader(res)
                        .into_iter::<EventResponse>()
                        .map(|event_response| Ok(event_response?)),
                ) as Box<dyn Iterator<Item = Result<EventResponse>>>
            })
    }

    /// List networks
    ///
    /// # API
    /// /networks
    pub fn list_networks(&self, filters: ListNetworkFilters) -> Result<Vec<Network>> {
        let path = if filters.is_empty() {
            "/networks".to_string()
        } else {
            let mut param = url::form_urlencoded::Serializer::new(String::new());
            param.append_pair("filters", &serde_json::to_string(&filters).unwrap());
            debug!("filter: {}", serde_json::to_string(&filters).unwrap());
            format!("/networks?{}", param.finish())
        };
        self.http_client()
            .get(self.headers(), &path)
            .and_then(api_result)
    }

    /// Inspect a network
    ///
    /// # API
    /// /networks/{id}
    pub fn inspect_network(
        &self,
        id: &str,
        verbose: Option<bool>,
        scope: Option<&str>,
    ) -> Result<Network> {
        let mut param = url::form_urlencoded::Serializer::new(String::new());
        param.append_pair("verbose", &verbose.unwrap_or(false).to_string());
        if let Some(scope) = scope {
            param.append_pair("scope", scope);
        }
        self.http_client()
            .get(
                self.headers(),
                &format!("/networks/{}?{}", id, param.finish()),
            )
            .and_then(api_result)
    }

    /// Remove a network
    ///
    /// # API
    /// /networks/{id}
    pub fn remove_network(&self, id: &str) -> Result<()> {
        self.http_client()
            .delete(self.headers(), &format!("/networks/{id}"))
            .and_then(no_content)
    }

    /// Create a network
    ///
    /// # API
    /// /networks/create
    pub fn create_network(&self, option: &NetworkCreateOptions) -> Result<CreateNetworkResponse> {
        let json_body = serde_json::to_string(&option)?;
        let mut headers = self.headers().clone();
        headers.insert(
            http::header::CONTENT_TYPE,
            "application/json".parse().unwrap(),
        );
        self.http_client()
            .post(&headers, "/networks/create", &json_body)
            .and_then(api_result)
    }

    /// Connect a container to a network
    ///
    /// # API
    /// /networks/{id}/connect
    pub fn connect_network(&self, id: &str, option: &NetworkConnectOptions) -> Result<()> {
        let json_body = serde_json::to_string(&option)?;
        let mut headers = self.headers().clone();
        headers.insert(
            http::header::CONTENT_TYPE,
            "application/json".parse().unwrap(),
        );
        self.http_client()
            .post(&headers, &format!("/networks/{id}/connect"), &json_body)
            .and_then(ignore_result)
    }

    /// Disconnect a container from a network
    ///
    /// # API
    /// /networks/{id}/disconnect
    pub fn disconnect_network(&self, id: &str, option: &NetworkDisconnectOptions) -> Result<()> {
        let json_body = serde_json::to_string(&option)?;
        let mut headers = self.headers().clone();
        headers.insert(
            http::header::CONTENT_TYPE,
            "application/json".parse().unwrap(),
        );
        self.http_client()
            .post(&headers, &format!("/networks/{id}/disconnect"), &json_body)
            .and_then(ignore_result)
    }

    /// Delete unused networks
    ///
    /// # API
    /// /networks/prune
    pub fn prune_networks(&self, filters: PruneNetworkFilters) -> Result<PruneNetworkResponse> {
        let path = if filters.is_empty() {
            "/networks/prune".to_string()
        } else {
            let mut param = url::form_urlencoded::Serializer::new(String::new());
            debug!("filters: {}", serde_json::to_string(&filters).unwrap());
            param.append_pair("filters", &serde_json::to_string(&filters).unwrap());
            format!("/networks/prune?{}", param.finish())
        };
        self.http_client()
            .post(self.headers(), &path, "")
            .and_then(api_result)
    }
}

impl HaveHttpClient for Docker {
    type Client = HyperClient;
    fn http_client(&self) -> &Self::Client {
        &self.client
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::convert::From;
    use std::env;
    use std::fs::{remove_file, File};
    use std::io::{self, Read, Write};
    use std::iter::{self, Iterator};
    use std::path::PathBuf;
    use std::thread;

    use chrono::Local;
    use rand::Rng;
    use tar::Builder as TarBuilder;

    use crate::container;

    #[test]
    fn test_ping() {
        let docker = Docker::connect_with_defaults().unwrap();
        docker.ping().unwrap();
    }

    #[test]
    fn test_system_info() {
        let docker = Docker::connect_with_defaults().unwrap();
        docker.system_info().unwrap();
    }

    #[test]
    fn test_version() {
        let docker = Docker::connect_with_defaults().unwrap();
        docker.version().unwrap();
    }

    #[test]
    fn test_events() {
        let docker = Docker::connect_with_defaults().unwrap();
        let _ = docker.events(None, None, None).unwrap();
    }

    fn double_stop_container(docker: &Docker, container: &str) {
        println!(
            "container info: {:?}",
            docker.container_info(container).unwrap()
        );
        docker.start_container(container).unwrap();
        docker
            .stop_container(container, Duration::from_secs(10))
            .unwrap();
        docker
            .stop_container(container, Duration::from_secs(10))
            .unwrap();
    }

    fn restart_container(docker: &Docker, container: &str) {
        docker.start_container(container).unwrap();
        docker
            .stop_container(container, Duration::from_secs(10))
            .unwrap();
        docker
            .restart_container(container, Duration::from_secs(10))
            .unwrap();
        docker
            .stop_container(container, Duration::from_secs(10))
            .unwrap();
    }

    fn stop_wait_container(docker: &Docker, container: &str) {
        docker.start_container(container).unwrap();
        docker.wait_container(container).unwrap();
    }

    fn head_file_container(docker: &Docker, container: &str) {
        let res = docker.head_file(container, Path::new("/bin/ls")).unwrap();
        assert_eq!(res.name, "ls");
        chrono::DateTime::parse_from_rfc3339(&res.mtime).unwrap();
    }

    fn stats_container(docker: &Docker, container: &str) {
        docker.start_container(container).unwrap();

        // one shot
        let one_stats = docker.stats(container, Some(false), Some(true)).unwrap();
        assert_eq!(one_stats.count(), 1);

        // stream
        let thr_stats = docker
            .stats(container, Some(true), Some(false))
            .unwrap()
            .take(3)
            .collect::<Vec<_>>();
        assert!(thr_stats.iter().all(Result::is_ok));

        docker
            .stop_container(container, Duration::from_secs(10))
            .unwrap();
    }

    fn wait_container(docker: &Docker, container: &str) {
        assert_eq!(
            docker.wait_container(container).unwrap(),
            ExitStatus::new(0)
        );
    }

    fn put_file_container(docker: &Docker, container: &str) {
        let temp_dir = env::temp_dir();
        let test_file = &temp_dir.join("test_file");

        gen_rand_file(test_file, 1024).unwrap();
        {
            let mut builder =
                TarBuilder::new(File::create(test_file.with_extension("tar")).unwrap());
            builder
                .append_file(
                    test_file.strip_prefix("/").unwrap(),
                    &mut File::open(test_file).unwrap(),
                )
                .unwrap();
        }
        assert!(matches!(
            docker
                .get_file(container, test_file)
                .map(|_| ())
                .unwrap_err(),
            Error::Docker(_) // not found
        ));
        docker
            .put_file(
                container,
                &test_file.with_extension("tar"),
                Path::new("/"),
                true,
            )
            .unwrap();
        docker
            .get_file(container, test_file)
            .unwrap()
            .unpack(temp_dir.join("put"))
            .unwrap();
        docker.wait_container(container).unwrap();

        assert!(equal_file(
            test_file,
            &temp_dir.join("put").join(test_file.file_name().unwrap())
        ));
    }

    fn log_container(docker: &Docker, container: &str) {
        docker.start_container(container).unwrap();

        let log_options = ContainerLogOptions {
            stdout: true,
            stderr: true,
            follow: true,
            ..ContainerLogOptions::default()
        };

        let mut log = docker.log_container(container, &log_options).unwrap();

        let log_all = log.output().unwrap();
        println!("log_all\n{log_all}");
    }

    fn connect_container(docker: &Docker, container_name: &str, container_id: &str, network: &str) {
        // docker run --net=network container
        docker.start_container(container_id).unwrap();
        let network_start = docker.inspect_network(network, None, None).unwrap();
        assert_eq!(&network_start.Containers[container_id].Name, container_name);

        // docker network disconnect network container
        docker
            .disconnect_network(
                network,
                &NetworkDisconnectOptions {
                    Container: container_id.to_owned(),
                    Force: false,
                },
            )
            .unwrap();

        let network_disconn = docker.inspect_network(network, None, None).unwrap();
        assert!(network_disconn.Containers.is_empty());

        // docker network connect network container
        // connecting with `docker network connect` command
        docker
            .connect_network(
                network,
                &NetworkConnectOptions {
                    Container: container_id.to_owned(),
                    EndpointConfig: EndpointConfig::default(),
                },
            )
            .unwrap();

        let network_conn = docker.inspect_network(network, None, None).unwrap();
        assert_eq!(&network_start.Id, &network_conn.Id);
        // .keys == ID of containers
        assert!(network_start
            .Containers
            .keys()
            .eq(network_conn.Containers.keys()));

        docker
            .stop_container(container_id, Duration::new(5, 0))
            .unwrap();
    }

    fn test_container(docker: &Docker, image: &str) {
        let mut next_id = {
            let mut id = 0;
            move || {
                let next = format!("test_container_{id}");
                id += 1;
                next
            }
        };
        println!("stop container");
        {
            let create = ContainerCreateOptions::new(image);

            let container = docker.create_container(Some(&next_id()), &create).unwrap();

            double_stop_container(docker, &container.id);

            docker
                .remove_container(&container.id, None, None, None)
                .unwrap();
        }
        println!("restart container");
        {
            let create = ContainerCreateOptions::new(image);

            let container = docker.create_container(Some(&next_id()), &create).unwrap();

            restart_container(docker, &container.id);

            docker
                .remove_container(&container.id, None, None, None)
                .unwrap();
        }
        println!("auto remove container");
        {
            let mut create = ContainerCreateOptions::new(image);
            let mut host_config = ContainerHostConfig::new();
            host_config.auto_remove(true);
            create.host_config(host_config);

            let container = docker.create_container(Some(&next_id()), &create).unwrap();

            stop_wait_container(docker, &container.id);

            // auto removed
            assert!(
                // 'no such container' or 'removel container in progress'
                docker
                    .remove_container(&container.id, None, None, None)
                    .is_err()
            );
        }
        println!("head file container");
        {
            let create = ContainerCreateOptions::new(image);

            let container = docker.create_container(Some(&next_id()), &create).unwrap();

            head_file_container(docker, &container.id);

            docker
                .remove_container(&container.id, None, None, None)
                .unwrap();
        }
        println!("stats container");
        {
            let create = ContainerCreateOptions::new(image);

            let container = docker.create_container(Some(&next_id()), &create).unwrap();

            stats_container(docker, &container.id);

            docker
                .remove_container(&container.id, None, None, None)
                .unwrap();
        }
        println!("exit 0");
        {
            let mut create = ContainerCreateOptions::new(image);
            create.cmd("ls".to_string());

            let container = docker.create_container(Some(&next_id()), &create).unwrap();

            wait_container(docker, &container.id);

            docker
                .remove_container(&container.id, None, None, None)
                .unwrap();
        }
        println!("put file");
        {
            let create = ContainerCreateOptions::new(image);

            let container = docker.create_container(Some(&next_id()), &create).unwrap();

            put_file_container(docker, &container.id);

            docker
                .remove_container(&container.id, None, None, None)
                .unwrap();
        }
        println!("logging container");
        {
            let mut create = ContainerCreateOptions::new(image);
            create.entrypoint(vec!["cat".into()]);
            create.cmd("/etc/motd".to_string());

            let container = docker.create_container(Some(&next_id()), &create).unwrap();

            log_container(docker, &container.id);

            docker
                .remove_container(&container.id, None, None, None)
                .unwrap();
        }
        println!("connect networks");
        {
            use std::collections::HashMap;
            let network_name = "dockworker_test_network_1";
            let network = docker
                .create_network(&NetworkCreateOptions::new(network_name))
                .unwrap();

            let mut create = ContainerCreateOptions::new(image);
            create
                .attach_stdout(false)
                .attach_stderr(false)
                .tty(true)
                .open_stdin(true);
            let mut config = HashMap::new();
            config.insert(network_name.to_owned(), EndpointConfig::default());
            create.networking_config(NetworkingConfig {
                endpoints_config: config.into(),
            });

            let container_name = next_id();
            let container = docker
                .create_container(Some(&container_name), &create)
                .unwrap();

            connect_container(docker, &container_name, &container.id, &network.Id);

            docker
                .remove_container(&container.id, None, None, None)
                .unwrap();

            docker.remove_network(&network.Id).unwrap();
        }
    }

    fn test_image_api(docker: &Docker, name: &str, tag: &str) {
        let mut filter = ContainerFilters::new();
        filter.name("test_container_");

        assert!(
            docker
                .list_containers(Some(true), None, Some(true), filter.clone())
                .unwrap()
                .is_empty(),
            "remove containers 'test_container_*'"
        );
        test_container(docker, &format!("{name}:{tag}"));
        assert!(docker
            .list_containers(Some(true), None, Some(true), filter)
            .unwrap()
            .is_empty());
    }

    fn test_image(docker: &Docker, name: &str, tag: &str) {
        docker.create_image(name, tag).unwrap().for_each(|st| {
            println!("{:?}", st.unwrap());
        });

        let image = format!("{name}:{tag}");
        let image_file = format!("dockworker_test_{name}_{tag}.tar");

        {
            let mut file = File::create(&image_file).unwrap();
            let mut res = docker.export_image(&image).unwrap();
            io::copy(&mut res, &mut file).unwrap();
        }

        docker.remove_image(&image, None, None).unwrap();
        docker.load_image(false, Path::new(&image_file)).unwrap();
        remove_file(&image_file).unwrap();

        test_image_api(docker, name, tag);

        docker
            .remove_image(&format!("{name}:{tag}"), None, None)
            .unwrap();
    }

    #[test]
    fn test_api() {
        let docker = Docker::connect_with_defaults().unwrap();

        let (name, tag) = ("alpine", "3.9");
        test_image(&docker, name, tag);
    }

    #[cfg(feature = "experimental")]
    #[test]
    fn test_container_checkpointing() {
        let docker = Docker::connect_with_defaults().unwrap();
        let (name, tag) = ("alpine", "3.10");
        with_image(&docker, name, tag, |name, tag| {
            let mut create = ContainerCreateOptions::new(&format!("{}:{}", name, tag));
            create.host_config(ContainerHostConfig::new());
            create.cmd("sleep".to_string());
            create.cmd("10000".to_string());
            let container = docker
                .create_container(Some("dockworker_checkpoint_test"), &create)
                .unwrap();
            docker.start_container(&container.id).unwrap();

            docker
                .checkpoint_container(
                    &container.id,
                    &CheckpointCreateOptions {
                        checkpoint_id: "v1".to_string(),
                        checkpoint_dir: None,
                        exit: Some(true),
                    },
                )
                .unwrap();

            assert_eq!(
                "v1".to_string(),
                docker
                    .list_container_checkpoints(&container.id, None)
                    .unwrap()[0]
                    .Name
            );

            thread::sleep(Duration::from_secs(1));

            docker
                .resume_container_from_checkpoint(&container.id, "v1", None)
                .unwrap();

            docker
                .stop_container(&container.id, Duration::new(0, 0))
                .unwrap();

            docker
                .delete_checkpoint(
                    &container.id,
                    &CheckpointDeleteOptions {
                        checkpoint_id: "v1".to_string(),
                        checkpoint_dir: None,
                    },
                )
                .unwrap();

            docker
                .remove_container("dockworker_checkpoint_test", None, None, None)
                .unwrap();
        })
    }

    // generate a file on path which is constructed from size chars alphanum seq
    fn gen_rand_file(path: &Path, size: usize) -> io::Result<()> {
        let mut rng = rand::thread_rng();
        let mut file = File::create(path)?;
        let vec: String = iter::repeat(())
            .map(|_| rng.sample(rand::distributions::Alphanumeric) as char)
            .take(size)
            .collect();
        file.write_all(vec.as_bytes())
    }

    fn equal_file(patha: &Path, pathb: &Path) -> bool {
        let filea = File::open(patha).unwrap();
        let fileb = File::open(pathb).unwrap();
        filea
            .bytes()
            .map(|e| e.ok())
            .eq(fileb.bytes().map(|e| e.ok()))
    }

    #[test]
    fn test_networks() {
        let docker = Docker::connect_with_defaults().unwrap();
        inspect_networks(&docker);
        prune_networks(&docker);
    }

    fn inspect_networks(docker: &Docker) {
        for network in &docker.list_networks(ListNetworkFilters::default()).unwrap() {
            let network = docker
                .inspect_network(&network.Id, Some(true), None)
                .unwrap();
            println!("network: {network:?}");
        }
        let create = NetworkCreateOptions::new("dockworker_test_network");
        let res = docker.create_network(&create).unwrap();
        let mut filter = ListNetworkFilters::default();
        filter.id(res.Id.as_str().into());
        assert_eq!(
            docker
                .list_networks(filter.clone())
                .unwrap()
                .iter()
                .filter(|n| n.Id == res.Id)
                .count(),
            1
        );
        docker.remove_network(&res.Id).unwrap();
        assert!(!docker
            .list_networks(filter)
            .unwrap()
            .iter()
            .any(|n| n.Id == res.Id));
    }

    fn prune_networks(docker: &Docker) {
        use crate::network::LabelFilter as F;
        use crate::network::NetworkCreateOptions as Net;
        use crate::network::PruneNetworkFilters as Prune;
        let mut create_nw_3 = Local::now();
        for i in 1..=6 {
            docker
                .create_network(
                    Net::new(&format!("nw_test_{i}"))
                        .label("alias", &format!("my-test-network-{i}"))
                        .label(&format!("test-network-{i}"), &i.to_string())
                        .label("not2", if i == 2 { "true" } else { "false" }),
                )
                .unwrap();

            thread::sleep(Duration::from_secs(1)); // drift timestamp in sec
            if i == 3 {
                create_nw_3 = Local::now();
            }
        }

        println!("filter network by label");
        assert_eq!(
            {
                let mut filter = Prune::default();
                filter.label(F::with(&[("test-network-1", None)]));
                &docker.prune_networks(filter).unwrap().networks_deleted
            },
            &["nw_test_1".to_owned()]
        );
        println!("filter network by negated label");
        assert_eq!(
            {
                let mut filter = Prune::default();
                filter.label_not(F::with(&[("not2", Some("false"))]));
                docker.prune_networks(filter).unwrap().networks_deleted
            },
            &["nw_test_2".to_owned()]
        );
        println!("filter network by timestamp");
        assert_eq!(
            {
                let mut filter = Prune::default();
                filter.until(vec![create_nw_3.timestamp()]);
                docker.prune_networks(filter).unwrap().networks_deleted
            },
            &["nw_test_3".to_owned()]
        );
        println!("filter network by label");
        assert_eq!(
            {
                let mut filter = Prune::default();
                filter.label(F::with(&[("test-network-4", Some("4"))]));
                docker.prune_networks(filter).unwrap().networks_deleted
            },
            &["nw_test_4".to_owned()]
        );
        println!("filter network by negated label");
        assert_eq!(
            {
                let mut filter = Prune::default();
                filter.label_not(F::with(&[("alias", Some("my-test-network-6"))]));
                docker.prune_networks(filter).unwrap().networks_deleted
            },
            &["nw_test_5".to_owned()]
        );
        println!("prune network");
        assert_eq!(
            docker
                .prune_networks(Prune::default())
                .unwrap()
                .networks_deleted,
            &["nw_test_6".to_owned()]
        );
    }

    /// This is executed after `docker-compose build iostream`
    #[test]
    #[ignore]
    fn attach_container() {
        use crate::signal::*;
        let docker = Docker::connect_with_defaults().unwrap();

        // expected files
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("docker/attach");
        let exps: &[&str; 2] = &["./sample/apache-2.0.txt", "./sample/bsd4.txt"];
        let image_name = "test-iostream:latest";

        let host_config = ContainerHostConfig::new();
        //host_config.auto_remove(true);
        let mut create = ContainerCreateOptions::new(image_name);
        create
            .cmd(exps[0].to_owned())
            .cmd(exps[1].to_owned())
            .host_config(host_config)
            .env("WAIT_BEFORE_CONTINUING=YES".to_string());

        let container = docker
            .create_container(Some("attach_container_test"), &create)
            .unwrap();
        docker.start_container(&container.id).unwrap();
        let res = docker
            .attach_container(&container.id, None, true, true, false, true, true)
            .unwrap();
        let cont: container::AttachContainer = res.into();

        // We've successfully attached, tell the container
        // to continue printing to stdout and stderr
        docker
            .kill_container(&container.id, Signal::from(SIGUSR1))
            .unwrap();

        // expected files
        let exp_stdout = File::open(root.join(exps[0])).unwrap();
        let exp_stderr = File::open(root.join(exps[1])).unwrap();

        assert!(exp_stdout
            .bytes()
            .map(|e| e.ok())
            .eq(cont.stdout.bytes().map(|e| e.ok())));
        assert!(exp_stderr
            .bytes()
            .map(|e| e.ok())
            .eq(cont.stderr.bytes().map(|e| e.ok())));

        docker.wait_container(&container.id).unwrap();
        docker
            .remove_container(&container.id, None, None, None)
            .unwrap();
    }

    /// This is executed after `docker-compose build iostream`
    #[test]
    #[ignore]
    fn exec_container() {
        let docker = Docker::connect_with_defaults().unwrap();

        // expected files
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("docker/attach");
        let exps: &[&str; 2] = &["./sample/apache-2.0.txt", "./sample/bsd4.txt"];
        let image_name = "test-iostream:latest";

        let host_config = ContainerHostConfig::new();
        let mut create = ContainerCreateOptions::new(image_name);
        create
            .entrypoint(vec!["sleep".to_owned()])
            .cmd("10".to_owned())
            .host_config(host_config);

        let container = docker
            .create_container(Some("exec_container_test"), &create)
            .unwrap();
        docker.start_container(&container.id).unwrap();

        let mut exec_config = CreateExecOptions::new();
        exec_config
            .cmd("./entrypoint.sh".to_owned())
            .cmd(exps[0].to_owned())
            .cmd(exps[1].to_owned());

        let exec_instance = docker.exec_container(&container.id, &exec_config).unwrap();
        let exec_start_config = StartExecOptions::new();
        let res = docker
            .start_exec(&exec_instance.id, &exec_start_config)
            .unwrap();
        let cont: container::AttachContainer = res.into();

        // expected files
        let exp_stdout = File::open(root.join(exps[0])).unwrap();
        let exp_stderr = File::open(root.join(exps[1])).unwrap();

        assert!(exp_stdout
            .bytes()
            .map(|e| e.ok())
            .eq(cont.stdout.bytes().map(|e| e.ok())));
        assert!(exp_stderr
            .bytes()
            .map(|e| e.ok())
            .eq(cont.stderr.bytes().map(|e| e.ok())));

        let exec_inspect = docker.exec_inspect(&exec_instance.id).unwrap();

        assert_eq!(exec_inspect.ExitCode, Some(0));
        assert_eq!(exec_inspect.Running, false);

        docker.wait_container(&container.id).unwrap();
        docker
            .remove_container(&container.id, None, None, None)
            .unwrap();
    }

    /// This is executed after `docker-compose build signal`
    #[test]
    #[ignore]
    fn signal_container() {
        use crate::signal::*;
        let docker = Docker::connect_with_defaults().unwrap();

        let image_name = "test-signal:latest";
        let host_config = ContainerHostConfig::new();
        let mut create = ContainerCreateOptions::new(image_name);
        create.host_config(host_config);

        let container = docker
            .create_container(Some("signal_container_test"), &create)
            .unwrap();
        docker.start_container(&container.id).unwrap();
        let res = docker
            .attach_container(&container.id, None, true, true, false, true, true)
            .unwrap();
        let cont: container::AttachContainer = res.into();
        let signals = [SIGHUP, SIGINT, SIGUSR1, SIGUSR2, SIGTERM];
        let signalstrs = vec![
            "HUP".to_string(),
            "INT".to_string(),
            "USR1".to_string(),
            "USR2".to_string(),
            "TERM".to_string(),
        ];

        signals.iter().for_each(|sig| {
            trace!("cause signal: {:?}", sig);
            docker
                .kill_container(&container.id, Signal::from(*sig))
                .ok();
        });

        let stdout_buffer = BufReader::new(cont.stdout);
        assert!(stdout_buffer
            .lines()
            .map(|line| line.unwrap())
            .eq(signalstrs));

        trace!("wait");
        assert_eq!(
            docker.wait_container(&container.id).unwrap(),
            ExitStatus::new(15)
        );

        trace!("remove container");
        docker
            .remove_container(&container.id, None, None, None)
            .unwrap();
    }

    // See https://github.com/hyperium/hyper/issues/2312
    #[test]
    #[ignore]
    fn workaround_hyper_hangup() {
        use std::sync::mpsc;

        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let docker = Docker::connect_with_defaults().unwrap();
            for _ in 0..1000 {
                let _events = docker.events(None, None, None).unwrap();
                tx.send(()).unwrap();
            }
        });
        for i in 0..1000 {
            assert_eq!(
                rx.recv_timeout(std::time::Duration::from_secs(15)),
                Ok(()),
                "i = {i}"
            );
        }
    }
}
