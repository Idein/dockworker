#![allow(clippy::bool_assert_comparison)]
use crate::container::{
    AttachResponseFrame, Container, ContainerFilters, ContainerInfo, ContainerStdioType, ExecInfo,
    ExitStatus,
};
pub use crate::credentials::{Credential, UserPassword};
use crate::errors::{DockerError, Error as DwError};
use crate::event::EventResponse;
use crate::filesystem::{FilesystemChange, XDockerContainerPathStat};
use crate::http_client::{HaveHttpClient, HttpClient};
use crate::hyper_client::HyperClient;
use crate::image::{FoundImage, Image, ImageFilters, ImageId, SummaryImage};
use crate::network::*;
use crate::options::*;
use crate::process::{Process, Top};
use crate::response::Response as DockerResponse;
use crate::signal::Signal;
use crate::stats::Stats;
use crate::system::{AuthToken, SystemInfo};
use crate::version::Version;
use base64::{engine::general_purpose, Engine as _};
use bytes::Bytes;
#[cfg(feature = "experimental")]
use checkpoint::{Checkpoint, CheckpointCreateOptions, CheckpointDeleteOptions};
use futures::stream::BoxStream;
use http::{HeaderMap, StatusCode};
use log::debug;
use serde::de::DeserializeOwned;
use std::env;
use std::path::{Path, PathBuf};
use std::time::Duration;

async fn into_aframe_stream(
    body: hyper::Body,
) -> Result<BoxStream<'static, Result<AttachResponseFrame, DwError>>, DwError> {
    use futures::stream::StreamExt;
    use futures::stream::TryStreamExt;
    let mut aread = tokio_util::io::StreamReader::new(
        body.map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, err)),
    );
    let mut buf = [0u8; 8];
    let src = async_stream::stream! {
        loop {
            use tokio::io::AsyncReadExt;
            if let Err(err) = aread.read_exact(&mut buf).await {
                if err.kind() == std::io::ErrorKind::UnexpectedEof {
                    break; // end of stream
                }
                log::error!("unexpected io error{:?}", err);
                yield Err(DwError::from(err));
                break;
            }
            // read body
            let mut frame_size_raw = &buf[4..];
            let frame_size = byteorder::ReadBytesExt::read_u32::<byteorder::BigEndian>(&mut frame_size_raw)
                .map_err(|e| DwError::Unknown{ message: format!("unexpeced buffer: {e:?}") })?;
            let mut frame = vec![0; frame_size as usize];
            if let Err(err) = aread.read_exact(&mut frame).await {
                if err.kind() == std::io::ErrorKind::UnexpectedEof {
                    break; // end of stream
                }
                log::error!("unexpected io error{:?}", err);
                yield Err(DwError::from(err));
                break;
            }
            match buf[0] {
                0 => {
                    yield Ok(AttachResponseFrame{ type_: ContainerStdioType::Stdin, frame });
                },
                1 => {
                    yield Ok(AttachResponseFrame{ type_: ContainerStdioType::Stdout, frame });
                },
                2 => {
                    yield Ok(AttachResponseFrame{ type_: ContainerStdioType::Stderr, frame });
                },
                n => {
                    log::error!("unexpected kind of chunk: {}", n);
                    yield Err(DwError::Unknown{ message: format!("unexpected kind of chunk: {}",n) });
                    break;
                }
            }
        }
    };
    Ok(src.boxed())
}

async fn into_docker_error(body: hyper::Body) -> Result<DockerError, DwError> {
    let body = hyper::body::to_bytes(body).await?;
    let err = serde_json::from_slice::<DockerError>(body.as_ref())?;
    Ok(err)
}

fn into_lines(body: hyper::Body) -> Result<BoxStream<'static, Result<String, DwError>>, DwError> {
    use futures::stream::StreamExt;
    use futures::stream::TryStreamExt;
    use tokio::io::AsyncBufReadExt;
    let aread = tokio_util::io::StreamReader::new(
        body.map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, err)),
    );
    let stream = tokio_stream::wrappers::LinesStream::new(aread.lines());
    let stream = stream.map_err(Into::into).boxed();
    Ok(stream)
}

pub fn into_jsonlines<T>(
    body: hyper::Body,
) -> Result<BoxStream<'static, Result<T, DwError>>, DwError>
where
    T: DeserializeOwned,
{
    use futures::StreamExt;
    let o = into_lines(body)?;
    let stream = o
        .map(|o| match o {
            Ok(o) => serde_json::from_str(&o).map_err(Into::into),
            Err(e) => Err(e),
        })
        .boxed();
    Ok(stream)
}

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
pub fn default_cert_path() -> Result<PathBuf, DwError> {
    let from_env = env::var("DOCKER_CERT_PATH").or_else(|_| env::var("DOCKER_CONFIG"));
    if let Ok(ref path) = from_env {
        Ok(PathBuf::from(path))
    } else {
        let home = dirs::home_dir().ok_or(DwError::NoCertPath)?;
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
#[derive(Debug, Clone)]
pub struct Docker {
    /// http client
    client: HyperClient,
    /// connection protocol
    #[allow(dead_code)]
    protocol: Protocol,
    /// http headers used for any requests
    headers: HeaderMap,
    /// access credential for accessing apis
    credential: std::sync::Arc<std::sync::Mutex<Option<Credential>>>,
}

/// Deserialize from json string
fn api_result<D: DeserializeOwned>(res: http::Response<Vec<u8>>) -> Result<D, DwError> {
    if res.status().is_success() {
        Ok(serde_json::from_slice::<D>(res.body())?)
    } else {
        Err(serde_json::from_slice::<DockerError>(res.body())?.into())
    }
}

/// Expect 204 NoContent
fn no_content(res: http::Response<Vec<u8>>) -> Result<(), DwError> {
    if res.status() == StatusCode::NO_CONTENT {
        Ok(())
    } else {
        Err(serde_json::from_slice::<DockerError>(res.body())?.into())
    }
}

/// Expect 204 NoContent or 304 NotModified
fn no_content_or_not_modified(res: http::Response<Vec<u8>>) -> Result<(), DwError> {
    if res.status() == StatusCode::NO_CONTENT || res.status() == StatusCode::NOT_MODIFIED {
        Ok(())
    } else {
        Err(serde_json::from_slice::<DockerError>(res.body())?.into())
    }
}

/// Ignore succeed response
///
/// Read whole response body, then ignore it.
fn ignore_result(res: http::Response<Vec<u8>>) -> Result<(), DwError> {
    if res.status().is_success() {
        Ok(())
    } else {
        Err(serde_json::from_slice::<DockerError>(res.body())?.into())
    }
}

impl Docker {
    fn new(client: HyperClient, protocol: Protocol) -> Self {
        Self {
            client,
            protocol,
            headers: HeaderMap::new(),
            credential: std::sync::Arc::new(std::sync::Mutex::new(None)),
        }
    }

    pub fn set_credential(&self, credential: Credential) {
        let mut o = self.credential.lock().unwrap();
        *o = Some(credential)
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
    pub fn connect_with_defaults() -> Result<Docker, DwError> {
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
            Err(DwError::UnsupportedScheme { host })
        }
    }

    /// This ensures that using a fully-qualified path
    ///
    /// e.g. unix://.... -- works.
    /// The unix socket provider expects a Path, so we don't need scheme.
    #[cfg(unix)]
    pub fn connect_with_unix(addr: &str) -> Result<Docker, DwError> {
        if let Some(addr) = addr.strip_prefix("unix://") {
            let client = HyperClient::connect_with_unix(addr);
            Ok(Docker::new(client, Protocol::Unix))
        } else {
            let client = HyperClient::connect_with_unix(addr);
            Ok(Docker::new(client, Protocol::Unix))
        }
    }

    #[cfg(not(unix))]
    pub fn connect_with_unix(addr: &str) -> Result<Docker, DwError> {
        Err(DwError::UnsupportedScheme {
            host: addr.to_owned(),
        }
        .into())
    }

    #[cfg(any(feature = "openssl", feature = "rustls"))]
    pub fn connect_with_ssl(
        addr: &str,
        key: &Path,
        cert: &Path,
        ca: &Path,
    ) -> Result<Docker, DwError> {
        let client = HyperClient::connect_with_ssl(addr, key, cert, ca).map_err(|err| {
            DwError::CouldNotConnect {
                addr: addr.to_owned(),
                source: err.into(),
            }
        })?;
        Ok(Docker::new(client, Protocol::Tcp))
    }

    #[cfg(not(any(feature = "openssl", feature = "rustls")))]
    pub fn connect_with_ssl(
        _addr: &str,
        _key: &Path,
        _cert: &Path,
        _ca: &Path,
    ) -> Result<Docker, DwError> {
        Err(DwError::SslDisabled)
    }

    /// Connect using unsecured HTTP.  This is strongly discouraged
    /// everywhere but on Windows when npipe support is not available.
    pub fn connect_with_http(addr: &str) -> Result<Docker, DwError> {
        let client =
            HyperClient::connect_with_http(addr).map_err(|err| DwError::CouldNotConnect {
                addr: addr.to_owned(),
                source: err.into(),
            })?;
        Ok(Docker::new(client, Protocol::Tcp))
    }

    /// List containers
    ///
    /// # API
    /// /containers/json
    pub async fn list_containers(
        &self,
        all: Option<bool>,
        limit: Option<u64>,
        size: Option<bool>,
        filters: ContainerFilters,
    ) -> Result<Vec<Container>, DwError> {
        let param = {
            let mut param = url::form_urlencoded::Serializer::new(String::new());
            param.append_pair("all", &(all.unwrap_or(false) as u64).to_string());
            if let Some(limit) = limit {
                param.append_pair("limit", &limit.to_string());
            }
            param.append_pair("size", &(size.unwrap_or(false) as u64).to_string());
            param.append_pair("filters", &serde_json::to_string(&filters).unwrap());
            param.finish()
        };
        debug!("filter: {}", serde_json::to_string(&filters).unwrap());
        let res = self
            .http_client()
            .get(self.headers(), &format!("/containers/json?{}", param))
            .await?;
        api_result(res).map_err(Into::into)
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
    pub async fn create_container(
        &self,
        name: Option<&str>,
        option: &ContainerCreateOptions,
    ) -> Result<CreateContainerResponse, DwError> {
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
        let res = self.http_client().post(&headers, &path, &json_body).await?;
        api_result(res).map_err(Into::into)
    }

    /// Start a container
    ///
    /// # API
    /// /containers/{id}/start
    pub async fn start_container(&self, id: &str) -> Result<(), DwError> {
        let res = self
            .http_client()
            .post(self.headers(), &format!("/containers/{id}/start"), "")
            .await?;
        no_content(res).map_err(Into::into)
    }

    /// Start a container from a checkpoint
    ///
    /// Using normal container start endpoint with preconfigured arguments
    ///
    /// # API
    /// /containers/{id}/start
    #[cfg(feature = "experimental")]
    pub async fn resume_container_from_checkpoint(
        &self,
        id: &str,
        checkpoint_id: &str,
        checkpoint_dir: Option<&str>,
    ) -> Result<(), DwError> {
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
            .await?;
        no_content(res).map_err(Into::into)
    }

    /// Stop a container
    ///
    /// # API
    /// /containers/{id}/stop
    pub async fn stop_container(&self, id: &str, timeout: Duration) -> Result<(), DwError> {
        let param = {
            let mut param = url::form_urlencoded::Serializer::new(String::new());
            param.append_pair("t", &timeout.as_secs().to_string());
            param.finish()
        };
        let res = self
            .http_client()
            .post(
                self.headers(),
                &format!("/containers/{}/stop?{}", id, param),
                "",
            )
            .await?;
        no_content_or_not_modified(res).map_err(Into::into)
    }

    /// Kill a container
    ///
    /// # API
    /// /containers/{id}/kill
    pub async fn kill_container(&self, id: &str, signal: Signal) -> Result<(), DwError> {
        let param = {
            let mut param = url::form_urlencoded::Serializer::new(String::new());
            param.append_pair("signal", &signal.as_i32().to_string());
            param.finish()
        };
        let res = self
            .http_client()
            .post(
                self.headers(),
                &format!("/containers/{}/kill?{}", id, param),
                "",
            )
            .await?;
        no_content(res).map_err(Into::into)
    }

    /// Restart a container
    ///
    /// # API
    /// /containers/{id}/restart
    pub async fn restart_container(&self, id: &str, timeout: Duration) -> Result<(), DwError> {
        let param = {
            let mut param = url::form_urlencoded::Serializer::new(String::new());
            param.append_pair("t", &timeout.as_secs().to_string());
            param.finish()
        };
        let res = self
            .http_client()
            .post(
                self.headers(),
                &format!("/containers/{}/restart?{}", id, param),
                "",
            )
            .await?;
        no_content(res).map_err(Into::into)
    }

    /// Attach to a container
    ///
    /// Attach to a container to read its output or send it input.
    ///
    /// # API
    /// /containers/{id}/attach
    #[allow(non_snake_case)]
    #[allow(clippy::too_many_arguments)]
    pub async fn attach_container(
        &self,
        id: &str,
        detachKeys: Option<&str>,
        logs: bool,
        stream: bool,
        stdin: bool,
        stdout: bool,
        stderr: bool,
    ) -> Result<BoxStream<'static, Result<AttachResponseFrame, DwError>>, DwError> {
        let param = {
            let mut param = url::form_urlencoded::Serializer::new(String::new());
            if let Some(keys) = detachKeys {
                param.append_pair("detachKeys", keys);
            }
            param.append_pair("logs", &logs.to_string());
            param.append_pair("stream", &stream.to_string());
            param.append_pair("stdin", &stdin.to_string());
            param.append_pair("stdout", &stdout.to_string());
            param.append_pair("stderr", &stderr.to_string());
            param.finish()
        };
        let res = self
            .http_client()
            .post_stream(
                self.headers(),
                &format!("/containers/{}/attach?{}", id, param),
                "",
            )
            .await?;
        if res.status().is_success() {
            into_aframe_stream(res.into_body()).await
        } else {
            Err(into_docker_error(res.into_body()).await?.into())
        }
    }

    /// List existing checkpoints from container
    ///
    /// Lists all snapshots made from the container in the specified directory.
    ///
    /// # API
    /// GET /containers/{id}/checkpoints
    #[cfg(feature = "experimental")]
    #[allow(non_snake_case)]
    pub async fn list_container_checkpoints(
        &self,
        id: &str,
        dir: Option<String>,
    ) -> Result<Vec<Checkpoint>, DwError> {
        let mut headers = self.headers().clone();
        headers.set::<ContentType>(ContentType::json());

        let mut param = url::form_urlencoded::Serializer::new(String::new());
        if let Some(_dir) = dir {
            param.append_pair("dir", &_dir);
        }

        let res = self
            .http_client()
            .get(
                &headers,
                &format!("/containers/{}/checkpoints?{}", id, param.finish()),
            )
            .await?;
        api_result(res).map_err(Into::into)
    }

    /// Create Checkpoint from current running container
    ///
    /// Create a snapshot of the container's current state.
    ///
    /// # API
    /// POST /containers/{id}/checkpoints
    #[cfg(feature = "experimental")]
    #[allow(non_snake_case)]
    pub async fn checkpoint_container(
        &self,
        id: &str,
        option: &CheckpointCreateOptions,
    ) -> Result<(), DwError> {
        let json_body = serde_json::to_string(&option)?;
        let mut headers = self.headers().clone();
        headers.set::<ContentType>(ContentType::json());
        let res = self
            .http_client()
            .post(
                &headers,
                &format!("/containers/{}/checkpoints", id),
                &json_body,
            )
            .await?;
        if res.status.is_success() && res.status == StatusCode::CREATED {
            Ok(())
        } else {
            Err(serde_json::from_reader::<_, DockerError>(res)?.into())
        }
    }

    /// Delete a checkpoint
    ///
    /// Delete a snapshot of a container specified by its name.
    ///
    /// # API
    /// DELETE /containers/{id}/checkpoints
    #[cfg(feature = "experimental")]
    #[allow(non_snake_case)]
    pub async fn delete_checkpoint(
        &self,
        id: &str,
        option: &CheckpointDeleteOptions,
    ) -> Result<(), DwError> {
        let mut headers = self.headers().clone();
        headers.set::<ContentType>(ContentType::json());

        let mut param = url::form_urlencoded::Serializer::new(String::new());
        let options = option.clone();
        if let Some(checkpoint_dir) = options.checkpoint_dir {
            param.append_pair("dir", &checkpoint_dir);
        }
        let res = self
            .http_client()
            .delete(
                &headers,
                &format!(
                    "/containers/{}/checkpoints/{}?{}",
                    id,
                    option.checkpoint_id,
                    param.finish()
                ),
            )
            .await?;
        no_content(res).map_err(Into::into)
    }

    /// Create Exec instance for a container
    ///
    /// Run a command inside a running container.
    ///
    /// # API
    /// /containers/{id}/exec
    #[allow(non_snake_case)]
    pub async fn exec_container(
        &self,
        id: &str,
        option: &CreateExecOptions,
    ) -> Result<CreateExecResponse, DwError> {
        let json_body = serde_json::to_string(&option)?;
        let mut headers = self.headers().clone();
        headers.insert(
            http::header::CONTENT_TYPE,
            "application/json".parse().unwrap(),
        );
        let res = self
            .http_client()
            .post(&headers, &format!("/containers/{id}/exec"), &json_body)
            .await?;
        api_result(res).map_err(Into::into)
    }

    /// Start an exec instance
    ///
    /// Starts a previously set up exec instance. If detach is true, this endpoint returns immediately after starting the command. Otherwise, it sets up an interactive session with the command.
    ///
    /// # API
    /// /exec/{id}/start
    #[allow(non_snake_case)]
    pub async fn start_exec(
        &self,
        id: &str,
        option: &StartExecOptions,
    ) -> Result<BoxStream<'static, Result<AttachResponseFrame, DwError>>, DwError> {
        let json_body = serde_json::to_string(&option)?;

        let mut headers = self.headers().clone();
        headers.insert(
            http::header::CONTENT_TYPE,
            "application/json".parse().unwrap(),
        );

        let res = self
            .http_client()
            .post_stream(&headers, &format!("/exec/{id}/start"), &json_body)
            .await?;
        if res.status().is_success() {
            into_aframe_stream(res.into_body()).await
        } else {
            Err(into_docker_error(res.into_body()).await?.into())
        }
    }

    /// Inspect an exec instance
    ///
    /// Return low-level information about an exec instance.
    ///
    /// # API
    /// /exec/{id}/json
    #[allow(non_snake_case)]
    pub async fn exec_inspect(&self, id: &str) -> Result<ExecInfo, DwError> {
        let res = self
            .http_client()
            .get(self.headers(), &format!("/exec/{id}/json"))
            .await?;
        api_result(res).map_err(Into::into)
    }

    /// Gets current logs and tails logs from a container
    ///
    /// # API
    /// /containers/{id}/logs
    pub async fn log_container(
        &self,
        id: &str,
        option: &ContainerLogOptions,
    ) -> Result<BoxStream<'static, Result<String, DwError>>, DwError> {
        let res = self
            .http_client()
            .get_stream(
                self.headers(),
                &format!("/containers/{}/logs?{}", id, option.to_url_params()),
            )
            .await?;
        if res.status().is_success() {
            into_lines(res.into_body())
        } else {
            Err(into_docker_error(res.into_body()).await?.into())
        }
    }

    /// List processes running inside a container
    ///
    /// # API
    /// /containers/{id}/top
    pub async fn container_top(&self, container_id: &str) -> Result<Top, DwError> {
        let res = self
            .http_client()
            .get(self.headers(), &format!("/containers/{container_id}/top"))
            .await?;
        api_result(res).map_err(Into::into)
    }

    pub async fn processes(&self, container_id: &str) -> Result<Vec<Process>, DwError> {
        let top = self.container_top(container_id).await?;
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
    pub async fn stats(
        &self,
        container_id: &str,
        stream: Option<bool>,
        oneshot: Option<bool>,
    ) -> Result<BoxStream<'static, Result<Stats, DwError>>, DwError> {
        let mut query = url::form_urlencoded::Serializer::new(String::new());
        query.append_pair("stream", &stream.unwrap_or(true).to_string());
        query.append_pair("one-shot", &oneshot.unwrap_or(false).to_string());
        let res = self
            .http_client()
            .get_stream(
                self.headers(),
                &format!("/containers/{}/stats?{}", container_id, query.finish()),
            )
            .await?;
        if res.status().is_success() {
            into_jsonlines(res.into_body())
        } else {
            Err(into_docker_error(res.into_body()).await?.into())
        }
    }

    /// Wait for a container
    ///
    /// # API
    /// /containers/{id}/wait
    pub async fn wait_container(&self, id: &str) -> Result<ExitStatus, DwError> {
        let res = self
            .http_client()
            .post(self.headers(), &format!("/containers/{id}/wait"), "")
            .await?;
        api_result(res).map_err(Into::into)
    }

    /// Remove a container
    ///
    /// # API
    /// /containers/{id}
    pub async fn remove_container(
        &self,
        id: &str,
        volume: Option<bool>,
        force: Option<bool>,
        link: Option<bool>,
    ) -> Result<(), DwError> {
        let param = {
            let mut param = url::form_urlencoded::Serializer::new(String::new());
            param.append_pair("v", &volume.unwrap_or(false).to_string());
            param.append_pair("force", &force.unwrap_or(false).to_string());
            param.append_pair("link", &link.unwrap_or(false).to_string());
            param.finish()
        };
        let res = self
            .http_client()
            .delete(self.headers(), &format!("/containers/{}?{}", id, param))
            .await?;
        no_content(res).map_err(Into::into)
    }

    /// Get an archive of a filesystem resource in a container
    ///
    /// # API
    /// /containers/{id}/archive
    pub async fn get_file(
        &self,
        id: &str,
        path: &Path,
    ) -> Result<BoxStream<'static, Result<Bytes, DwError>>, DwError> {
        debug!("get_file({}, {})", id, path.display());
        let param = {
            let mut param = url::form_urlencoded::Serializer::new(String::new());
            param.append_pair("path", path.to_str().unwrap_or("")); // FIXME: cause an invalid path error
            param.finish()
        };
        let res = self
            .http_client()
            .get_stream(
                self.headers(),
                &format!("/containers/{}/archive?{}", id, param),
            )
            .await?;
        if res.status().is_success() {
            use futures::stream::StreamExt;
            use futures::stream::TryStreamExt;
            Ok(res.into_body().map_err(DwError::from).boxed())
        } else {
            Err(into_docker_error(res.into_body()).await?.into())
        }
    }

    /// Get information about files in a container
    ///
    /// # API
    /// /containers/{id}/archive
    pub async fn head_file(
        &self,
        id: &str,
        path: &Path,
    ) -> Result<XDockerContainerPathStat, DwError> {
        debug!("head_file({}, {})", id, path.display());
        let param = {
            let mut param = url::form_urlencoded::Serializer::new(String::new());
            param.append_pair("path", path.to_str().unwrap_or(""));
            param.finish()
        };
        let res = self
            .http_client()
            .head(
                self.headers(),
                &format!("/containers/{}/archive?{}", id, param),
            )
            .await?;
        let stat_base64: &str = res
            .get("X-Docker-Container-Path-Stat")
            .map(|h| h.to_str().unwrap_or(""))
            .unwrap_or("");
        let bytes = general_purpose::STANDARD
            .decode(stat_base64)
            .map_err(|err| DwError::ParseError {
                input: String::from(stat_base64),
                source: err,
            })?;
        let path_stat: XDockerContainerPathStat = serde_json::from_slice(&bytes)?;
        Ok(path_stat)
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
    pub async fn put_file(
        &self,
        id: &str,
        src: &Path,
        dst: &Path,
        noOverwriteDirNonDir: bool,
    ) -> Result<(), DwError> {
        debug!(
            "put_file({}, {}, {}, {})",
            id,
            src.display(),
            dst.display(),
            noOverwriteDirNonDir
        );
        let param = {
            let mut param = url::form_urlencoded::Serializer::new(String::new());
            param.append_pair("path", &dst.to_string_lossy());
            param.append_pair("noOverwriteDirNonDir", &noOverwriteDirNonDir.to_string());
            param.finish()
        };
        let res = self
            .http_client()
            .put_file(
                self.headers(),
                &format!("/containers/{}/archive?{}", id, param),
                src,
            )
            .await?;
        ignore_result(res).map_err(Into::into)
    }

    /// Build an image from a tar archive with a Dockerfile in it.
    ///
    /// # API
    /// /build?
    pub async fn build_image(
        &self,
        options: ContainerBuildOptions,
        tar_path: &Path,
    ) -> Result<BoxStream<'static, Result<DockerResponse, DwError>>, DwError> {
        let mut headers = self.headers().clone();
        headers.insert(
            http::header::CONTENT_TYPE,
            "application/x-tar".parse().unwrap(),
        );
        let res = self
            .http_client()
            .post_file_stream(
                &headers,
                &format!("/build?{}", options.to_url_params()),
                tar_path,
            )
            .await?;
        if res.status().is_success() {
            into_jsonlines(res.into_body())
        } else {
            Err(into_docker_error(res.into_body()).await?.into())
        }
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
    pub async fn create_image(
        &self,
        image: &str,
        tag: &str,
    ) -> Result<BoxStream<'static, Result<DockerResponse, DwError>>, DwError> {
        let param = {
            let mut param = url::form_urlencoded::Serializer::new(String::new());
            param.append_pair("fromImage", image);
            param.append_pair("tag", tag);
            param.finish()
        };

        let mut headers = self.headers().clone();
        if let Some(ref credential) = self.credential.lock().unwrap().as_ref() {
            headers.insert(
                "X-Registry-Auth",
                general_purpose::STANDARD
                    .encode(serde_json::to_string(credential).unwrap().as_bytes())
                    .parse()
                    .unwrap(),
            );
        }
        let res = self
            .http_client()
            .post_stream(&headers, &format!("/images/create?{}", param), "")
            .await?;
        if res.status().is_success() {
            into_jsonlines(res.into_body())
        } else {
            Err(into_docker_error(res.into_body()).await?.into())
        }
    }

    /// Inspect an image
    ///
    /// # API
    /// /images/{name}/json
    ///
    pub async fn inspect_image(&self, name: &str) -> Result<Image, DwError> {
        let res = self
            .http_client()
            .get(self.headers(), &format!("/images/{name}/json"))
            .await?;
        api_result(res).map_err(Into::into)
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
    pub async fn push_image(&self, name: &str, tag: &str) -> Result<(), DwError> {
        let param = {
            let mut param = url::form_urlencoded::Serializer::new(String::new());
            param.append_pair("tag", tag);
            param.finish()
        };
        let mut headers = self.headers().clone();
        if let Some(ref credential) = self.credential.lock().unwrap().as_ref() {
            headers.insert(
                "X-Registry-Auth",
                general_purpose::STANDARD
                    .encode(serde_json::to_string(credential).unwrap().as_bytes())
                    .parse()
                    .unwrap(),
            );
        }
        let res = self
            .http_client()
            .post(&headers, &format!("/images/{}/push?{}", name, param), "")
            .await?;
        ignore_result(res).map_err(Into::into)
    }

    /// Remove an image
    ///
    /// # API
    /// /images/{name}
    ///
    pub async fn remove_image(
        &self,
        name: &str,
        force: Option<bool>,
        noprune: Option<bool>,
    ) -> Result<Vec<RemovedImage>, DwError> {
        let param = {
            let mut param = url::form_urlencoded::Serializer::new(String::new());
            param.append_pair("force", &force.unwrap_or(false).to_string());
            param.append_pair("noprune", &noprune.unwrap_or(false).to_string());
            param.finish()
        };
        let res = self
            .http_client()
            .delete(self.headers(), &format!("/images/{}?{}", name, param))
            .await?;
        api_result(res).map_err(Into::into)
    }

    /// Delete unused images
    ///
    /// # API
    /// /images/prune
    pub async fn prune_image(&self, dangling: bool) -> Result<PrunedImages, DwError> {
        debug!("start pruning...dangling? {}", &dangling);
        let param = {
            let mut param = url::form_urlencoded::Serializer::new(String::new());
            param.append_pair(
                "filters",
                &format!(r#"{{ "dangling": {{ "{dangling}": true }} }}"#),
            );
            param.finish()
        };
        let res = self
            .http_client()
            .post(self.headers(), &format!("/images/prune?{}", param), "")
            .await?;
        api_result(res).map_err(Into::into)
    }

    /// History of an image
    ///
    /// # API
    /// /images/{name}/history
    ///
    pub async fn history_image(&self, name: &str) -> Result<Vec<ImageLayer>, DwError> {
        let res = self
            .http_client()
            .get(self.headers(), &format!("/images/{name}/history"))
            .await?;
        api_result(res)
            .map_err(Into::into)
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
    pub async fn images(&self, all: bool) -> Result<Vec<SummaryImage>, DwError> {
        let res = self
            .http_client()
            .get(self.headers(), &format!("/images/json?a={}", all as u32))
            .await?;
        api_result(res).map_err(Into::into)
    }

    /// Search for an image on Docker Hub.
    ///
    /// # API
    /// /images/search
    pub async fn search_images(
        &self,
        term: &str,
        limit: Option<u64>,
        filters: ImageFilters,
    ) -> Result<Vec<FoundImage>, DwError> {
        let mut param = url::form_urlencoded::Serializer::new(String::new());
        param.append_pair("term", term);
        if let Some(limit) = limit {
            param.append_pair("limit", &limit.to_string());
        }
        param.append_pair("filters", &serde_json::to_string(&filters).unwrap());
        let res = self
            .http_client()
            .get(
                self.headers(),
                &format!("/images/search?{}", param.finish()),
            )
            .await?;
        api_result(res).map_err(Into::into)
    }

    /// Get a tarball containing all images and metadata for a repository
    ///
    /// # API
    /// /images/{name}/get
    pub async fn export_image(
        &self,
        name: &str,
    ) -> Result<BoxStream<'static, Result<Bytes, DwError>>, DwError> {
        let res = self
            .http_client()
            .get_stream(self.headers(), &format!("/images/{name}/get"))
            .await?;
        if res.status().is_success() {
            use futures::stream::StreamExt;
            use futures::stream::TryStreamExt;
            Ok(res.into_body().map_err(Into::into).boxed())
        } else {
            Err(into_docker_error(res.into_body()).await?.into())
        }
    }

    /// Import images
    ///
    /// # Summary
    /// Load a set of images and tags into a repository
    ///
    /// # API
    /// /images/load
    pub async fn load_image(&self, quiet: bool, path: &Path) -> Result<ImageId, DwError> {
        let mut headers = self.headers().clone();
        headers.insert(
            http::header::CONTENT_TYPE,
            "application/x-tar".parse().unwrap(),
        );
        let res = self
            .http_client()
            .post_file(&headers, &format!("/images/load?quiet={quiet}"), path)
            .await?;
        if !res.status().is_success() {
            return Err(serde_json::from_slice::<DockerError>(res.body())?.into());
        }
        let path = path.to_owned();
        tokio::task::spawn_blocking(move || {
            let file = std::fs::File::open(path)?;
            let mut ar = tar::Archive::new(file);
            for entry in ar.entries()?.filter_map(|e| e.ok()) {
                let path = entry.path()?;
                // looking for file name like XXXXXXXXXXXXXX.json
                if path.extension() == Some(std::ffi::OsStr::new("json"))
                    && path != Path::new("manifest.json")
                {
                    let stem = path.file_stem().unwrap(); // contains .json
                    let id = stem.to_str().ok_or(DwError::Unknown {
                        message: format!("convert to String: {stem:?}"),
                    })?;
                    return Ok(ImageId::new(id.to_string()));
                }
            }
            Err(DwError::Unknown {
                message: "no expected file: XXXXXX.json".to_owned(),
            })
        })
        .await
        .expect("join error")
    }

    /// Check auth configuration
    ///
    /// # API
    /// /auth
    ///
    /// # NOTE
    /// In some cases, docker daemon returns an empty token with `200 Ok`.
    /// The empty token could not be used for authenticating users.
    pub async fn auth(
        &self,
        username: &str,
        password: &str,
        email: &str,
        serveraddress: &str,
    ) -> Result<AuthToken, DwError> {
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
        let res = self
            .http_client()
            .post(&headers, "/auth", &json_body)
            .await?;
        api_result(res).map_err(Into::into)
    }

    /// Get system information
    ///
    /// # API
    /// /info
    pub async fn system_info(&self) -> Result<SystemInfo, DwError> {
        let res = self.http_client().get(self.headers(), "/info").await?;
        api_result(res).map_err(Into::into)
    }

    /// Inspect about a container
    ///
    /// # API
    /// /containers/{id}/json
    pub async fn container_info(&self, container_id: &str) -> Result<ContainerInfo, DwError> {
        let res = self
            .http_client()
            .get(self.headers(), &format!("/containers/{container_id}/json"))
            .await?;
        api_result(res).map_err(Into::into)
    }

    /// Get changes on a container's filesystem.
    ///
    /// (This is the same as `docker container diff` command.)
    ///
    /// # API
    /// /containers/{id}/changes
    pub async fn filesystem_changes(
        &self,
        container_id: &str,
    ) -> Result<Vec<FilesystemChange>, DwError> {
        let res = self
            .http_client()
            .get(
                self.headers(),
                &format!("/containers/{container_id}/changes"),
            )
            .await?;
        api_result(res).map_err(Into::into)
    }

    /// Export a container
    ///
    /// # Summary
    /// Returns a pointer to tar archive stream.
    ///
    /// # API
    /// /containers/{id}/export
    pub async fn export_container(
        &self,
        container_id: &str,
    ) -> Result<BoxStream<'static, Result<Bytes, DwError>>, DwError> {
        let res = self
            .http_client()
            .get_stream(
                self.headers(),
                &format!("/containers/{container_id}/export"),
            )
            .await?;
        if res.status().is_success() {
            use futures::stream::StreamExt;
            use futures::stream::TryStreamExt;
            Ok(res.into_body().map_err(Into::into).boxed())
        } else {
            Err(into_docker_error(res.into_body()).await?.into())
        }
    }

    /// Test if the server is accessible
    ///
    /// # API
    /// /_ping
    pub async fn ping(&self) -> Result<(), DwError> {
        let res = self.http_client().get(self.headers(), "/_ping").await?;
        if res.status().is_success() {
            let buf = String::from_utf8(res.into_body().to_vec()).unwrap();
            assert_eq!(&buf, "OK");
            Ok(())
        } else {
            Err(serde_json::from_slice::<DockerError>(res.body())?.into())
        }
    }

    /// Get version and various information
    ///
    /// # API
    /// /version
    pub async fn version(&self) -> Result<Version, DwError> {
        let res = self.http_client().get(self.headers(), "/version").await?;
        api_result(res).map_err(Into::into)
    }

    /// Get monitor events
    ///
    /// # API
    /// /events
    pub async fn events(
        &self,
        since: Option<u64>,
        until: Option<u64>,
        filters: Option<EventFilters>,
    ) -> Result<BoxStream<'static, Result<EventResponse, DwError>>, DwError> {
        let param = {
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
            param.finish()
        };

        let res = self
            .http_client()
            .get_stream(self.headers(), &format!("/events?{}", param))
            .await?;
        into_jsonlines(res.into_body())
    }

    /// List networks
    ///
    /// # API
    /// /networks
    pub async fn list_networks(
        &self,
        filters: ListNetworkFilters,
    ) -> Result<Vec<Network>, DwError> {
        let path = if filters.is_empty() {
            "/networks".to_string()
        } else {
            let mut param = url::form_urlencoded::Serializer::new(String::new());
            param.append_pair("filters", &serde_json::to_string(&filters).unwrap());
            debug!("filter: {}", serde_json::to_string(&filters).unwrap());
            format!("/networks?{}", param.finish())
        };
        let res = self.http_client().get(self.headers(), &path).await?;
        api_result(res).map_err(Into::into)
    }

    /// Inspect a network
    ///
    /// # API
    /// /networks/{id}
    pub async fn inspect_network(
        &self,
        id: &str,
        verbose: Option<bool>,
        scope: Option<&str>,
    ) -> Result<Network, DwError> {
        let param = {
            let mut param = url::form_urlencoded::Serializer::new(String::new());
            param.append_pair("verbose", &verbose.unwrap_or(false).to_string());
            if let Some(scope) = scope {
                param.append_pair("scope", scope);
            }
            param.finish()
        };
        let res = self
            .http_client()
            .get(self.headers(), &format!("/networks/{}?{}", id, param))
            .await?;
        api_result(res).map_err(Into::into)
    }

    /// Remove a network
    ///
    /// # API
    /// /networks/{id}
    pub async fn remove_network(&self, id: &str) -> Result<(), DwError> {
        let res = self
            .http_client()
            .delete(self.headers(), &format!("/networks/{id}"))
            .await?;
        no_content(res).map_err(Into::into)
    }

    /// Create a network
    ///
    /// # API
    /// /networks/create
    pub async fn create_network(
        &self,
        option: &NetworkCreateOptions,
    ) -> Result<CreateNetworkResponse, DwError> {
        let json_body = serde_json::to_string(&option)?;
        let mut headers = self.headers().clone();
        headers.insert(
            http::header::CONTENT_TYPE,
            "application/json".parse().unwrap(),
        );
        let res = self
            .http_client()
            .post(&headers, "/networks/create", &json_body)
            .await?;
        api_result(res).map_err(Into::into)
    }

    /// Connect a container to a network
    ///
    /// # API
    /// /networks/{id}/connect
    pub async fn connect_network(
        &self,
        id: &str,
        option: &NetworkConnectOptions,
    ) -> Result<(), DwError> {
        let json_body = serde_json::to_string(&option)?;
        let mut headers = self.headers().clone();
        headers.insert(
            http::header::CONTENT_TYPE,
            "application/json".parse().unwrap(),
        );
        let res = self
            .http_client()
            .post(&headers, &format!("/networks/{id}/connect"), &json_body)
            .await?;
        ignore_result(res).map_err(Into::into)
    }

    /// Disconnect a container from a network
    ///
    /// # API
    /// /networks/{id}/disconnect
    pub async fn disconnect_network(
        &self,
        id: &str,
        option: &NetworkDisconnectOptions,
    ) -> Result<(), DwError> {
        let json_body = serde_json::to_string(&option)?;
        let mut headers = self.headers().clone();
        headers.insert(
            http::header::CONTENT_TYPE,
            "application/json".parse().unwrap(),
        );
        let res = self
            .http_client()
            .post(&headers, &format!("/networks/{id}/disconnect"), &json_body)
            .await?;
        ignore_result(res).map_err(Into::into)
    }

    /// Delete unused networks
    ///
    /// # API
    /// /networks/prune
    pub async fn prune_networks(
        &self,
        filters: PruneNetworkFilters,
    ) -> Result<PruneNetworkResponse, DwError> {
        let path = if filters.is_empty() {
            "/networks/prune".to_string()
        } else {
            let mut param = url::form_urlencoded::Serializer::new(String::new());
            debug!("filters: {}", serde_json::to_string(&filters).unwrap());
            param.append_pair("filters", &serde_json::to_string(&filters).unwrap());
            format!("/networks/prune?{}", param.finish())
        };
        let res = self.http_client().post(self.headers(), &path, "").await?;
        api_result(res).map_err(Into::into)
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
    use std::iter::{self, Iterator};
    use std::path::PathBuf;

    use chrono::Local;
    use futures::StreamExt;
    use http::request;
    use log::trace;
    use rand::Rng;

    async fn read_bytes_stream_to_end(src: BoxStream<'static, Result<Bytes, DwError>>) -> Vec<u8> {
        use futures::stream::TryStreamExt;
        let src = src.map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, err));
        let mut aread = tokio_util::io::StreamReader::new(src);
        let mut buf = vec![];
        use tokio::io::AsyncReadExt;
        aread.read_to_end(&mut buf).await.unwrap();
        buf
    }

    async fn read_frame_all(
        mut src: BoxStream<'static, Result<AttachResponseFrame, DwError>>,
    ) -> Result<(Vec<u8>, Vec<u8>, Vec<u8>), DwError> {
        let mut stdout_buf = vec![];
        let mut stdin_buf = vec![];
        let mut stderr_buf = vec![];
        use futures::stream::StreamExt;
        while let Some(mut stdio) = src.next().await.transpose()? {
            match stdio.type_ {
                ContainerStdioType::Stdin => {
                    stdin_buf.append(&mut stdio.frame);
                }
                ContainerStdioType::Stdout => {
                    stdout_buf.append(&mut stdio.frame);
                }
                ContainerStdioType::Stderr => {
                    stderr_buf.append(&mut stdio.frame);
                }
            }
        }
        Ok((stdin_buf, stdout_buf, stderr_buf))
    }

    async fn read_file(path: PathBuf) -> Vec<u8> {
        let mut file = tokio::fs::File::open(path).await.unwrap();
        let mut buf = vec![];
        use tokio::io::AsyncReadExt;
        file.read_to_end(&mut buf).await.unwrap();
        buf
    }

    #[tokio::test]
    async fn test_ping() {
        let docker = Docker::connect_with_defaults().unwrap();
        docker.ping().await.unwrap();
    }

    #[tokio::test]
    async fn test_system_info() {
        let docker = Docker::connect_with_defaults().unwrap();
        docker.system_info().await.unwrap();
    }

    #[tokio::test]
    async fn test_version() {
        let docker = Docker::connect_with_defaults().unwrap();
        docker.version().await.unwrap();
    }

    #[tokio::test]
    async fn test_events() {
        let docker = Docker::connect_with_defaults().unwrap();
        let _ = docker.events(None, None, None).await.unwrap();
    }

    async fn double_stop_container(docker: &Docker, container: &str) {
        let info = docker.container_info(container).await.unwrap();
        println!("container info: {info:?}");
        docker.start_container(container).await.unwrap();
        docker
            .stop_container(container, Duration::from_secs(10))
            .await
            .unwrap();
        docker
            .stop_container(container, Duration::from_secs(10))
            .await
            .unwrap();
    }

    async fn restart_container(docker: &Docker, container: &str) {
        docker.start_container(container).await.unwrap();
        docker
            .stop_container(container, Duration::from_secs(10))
            .await
            .unwrap();
        docker
            .restart_container(container, Duration::from_secs(10))
            .await
            .unwrap();
        docker
            .stop_container(container, Duration::from_secs(10))
            .await
            .unwrap();
    }

    async fn stop_wait_container(docker: &Docker, container: &str) {
        docker.start_container(container).await.unwrap();
        docker.wait_container(container).await.unwrap();
    }

    async fn head_file_container(docker: &Docker, container: &str) {
        let res = docker
            .head_file(container, Path::new("/bin/ls"))
            .await
            .unwrap();
        assert_eq!(res.name, "ls");
        chrono::DateTime::parse_from_rfc3339(&res.mtime).unwrap();
    }

    async fn stats_container(docker: &Docker, container: &str) {
        docker.start_container(container).await.unwrap();

        // one shot
        let one_stats = docker
            .stats(container, Some(false), Some(true))
            .await
            .unwrap();
        use futures::StreamExt;
        let one_stats = one_stats.collect::<Vec<_>>().await;
        assert_eq!(one_stats.len(), 1);

        // stream
        let thr_stats = docker
            .stats(container, Some(true), Some(false))
            .await
            .unwrap()
            .take(3)
            .collect::<Vec<_>>()
            .await;
        assert!(thr_stats.iter().all(Result::is_ok));

        docker
            .stop_container(container, Duration::from_secs(10))
            .await
            .unwrap();
    }

    async fn wait_container(docker: &Docker, container: &str) {
        let status = docker.wait_container(container).await.unwrap();
        assert_eq!(status, ExitStatus::new(0));
    }

    async fn put_file_container(docker: &Docker, container: &str) {
        let temp_dir = env::temp_dir();
        let test_file = temp_dir.join("test_file");

        gen_rand_file(&test_file, 1024).await.unwrap();
        // prepare test file
        tokio::task::spawn_blocking({
            let test_file = test_file.clone();
            move || {
                let file = std::fs::File::create(test_file.with_extension("tar")).unwrap();
                let mut builder = tar::Builder::new(file);
                let mut file2 = std::fs::File::open(&test_file).unwrap();
                builder
                    .append_file(test_file.strip_prefix("/").unwrap(), &mut file2)
                    .unwrap();
            }
        })
        .await
        .unwrap();
        let res = docker.get_file(container, &test_file).await;
        assert!(matches!(
            res.map(|_| ()).unwrap_err(),
            DwError::Docker(_) // not found
        ));
        docker
            .put_file(
                container,
                &test_file.with_extension("tar"),
                Path::new("/"),
                true,
            )
            .await
            .unwrap();
        let src = docker.get_file(container, &test_file).await.unwrap();
        let buf = read_bytes_stream_to_end(src).await;
        let temp_dir_put = temp_dir.join("put");
        tokio::task::spawn_blocking(move || {
            let cur = std::io::Cursor::new(buf);
            tar::Archive::new(cur).unpack(&temp_dir_put).unwrap();
        })
        .await
        .unwrap();
        docker.wait_container(container).await.unwrap();
        let is_eq = equal_file(
            &test_file,
            &temp_dir.join("put").join(test_file.file_name().unwrap()),
        )
        .await;
        assert!(is_eq);
    }

    async fn log_container(docker: &Docker, container: &str) {
        docker.start_container(container).await.unwrap();

        let log_options = ContainerLogOptions {
            stdout: true,
            stderr: true,
            follow: true,
            ..ContainerLogOptions::default()
        };

        let log = docker.log_container(container, &log_options).await.unwrap();
        use futures::stream::StreamExt;
        let log_all = log.collect::<Vec<Result<String, _>>>().await;
        let log_all = log_all.into_iter().collect::<Result<Vec<_>, _>>().unwrap();
        let log_all = log_all.join("\n");

        println!("log_all\n{log_all}");
    }

    async fn connect_container(
        docker: &Docker,
        container_name: &str,
        container_id: &str,
        network: &str,
    ) {
        // docker run --net=network container
        docker.start_container(container_id).await.unwrap();
        let network_start = docker.inspect_network(network, None, None).await.unwrap();
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
            .await
            .unwrap();

        let network_disconn = docker.inspect_network(network, None, None).await.unwrap();
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
            .await
            .unwrap();

        let network_conn = docker.inspect_network(network, None, None).await.unwrap();
        assert_eq!(&network_start.Id, &network_conn.Id);
        // .keys == ID of containers
        let is_eq = network_start
            .Containers
            .keys()
            .eq(network_conn.Containers.keys());
        assert!(is_eq);

        docker
            .stop_container(container_id, Duration::new(5, 0))
            .await
            .unwrap();
    }

    async fn test_container(docker: &Docker, image: &str) {
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

            let container = docker
                .create_container(Some(&next_id()), &create)
                .await
                .unwrap();

            double_stop_container(docker, &container.id).await;

            docker
                .remove_container(&container.id, None, None, None)
                .await
                .unwrap();
        }
        println!("restart container");
        {
            let create = ContainerCreateOptions::new(image);

            let container = docker
                .create_container(Some(&next_id()), &create)
                .await
                .unwrap();

            restart_container(docker, &container.id).await;

            docker
                .remove_container(&container.id, None, None, None)
                .await
                .unwrap();
        }
        println!("auto remove container");
        {
            let mut create = ContainerCreateOptions::new(image);
            let mut host_config = ContainerHostConfig::new();
            host_config.auto_remove(true);
            create.host_config(host_config);

            let container = docker
                .create_container(Some(&next_id()), &create)
                .await
                .unwrap();

            stop_wait_container(docker, &container.id).await;

            // auto removed
            // 'no such container' or 'removel container in progress'
            let res = docker
                .remove_container(&container.id, None, None, None)
                .await;
            assert!(res.is_err());
        }
        println!("head file container");
        {
            let create = ContainerCreateOptions::new(image);

            let container = docker
                .create_container(Some(&next_id()), &create)
                .await
                .unwrap();

            head_file_container(docker, &container.id).await;

            docker
                .remove_container(&container.id, None, None, None)
                .await
                .unwrap();
        }
        println!("stats container");
        {
            let create = ContainerCreateOptions::new(image);

            let container = docker
                .create_container(Some(&next_id()), &create)
                .await
                .unwrap();

            stats_container(docker, &container.id).await;

            docker
                .remove_container(&container.id, None, None, None)
                .await
                .unwrap();
        }
        println!("exit 0");
        {
            let mut create = ContainerCreateOptions::new(image);
            create.cmd("ls".to_string());

            let container = docker
                .create_container(Some(&next_id()), &create)
                .await
                .unwrap();

            wait_container(docker, &container.id).await;

            docker
                .remove_container(&container.id, None, None, None)
                .await
                .unwrap();
        }
        println!("put file");
        {
            let create = ContainerCreateOptions::new(image);

            let container = docker
                .create_container(Some(&next_id()), &create)
                .await
                .unwrap();

            put_file_container(docker, &container.id).await;

            docker
                .remove_container(&container.id, None, None, None)
                .await
                .unwrap();
        }
        println!("logging container");
        {
            let mut create = ContainerCreateOptions::new(image);
            create.entrypoint(vec!["cat".into()]);
            create.cmd("/etc/motd".to_string());

            let container = docker
                .create_container(Some(&next_id()), &create)
                .await
                .unwrap();

            log_container(docker, &container.id).await;

            docker
                .remove_container(&container.id, None, None, None)
                .await
                .unwrap();
        }
        println!("connect networks");
        {
            use std::collections::HashMap;
            let network_name = "dockworker_test_network_1";
            let network = docker
                .create_network(&NetworkCreateOptions::new(network_name))
                .await
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
                .await
                .unwrap();

            connect_container(docker, &container_name, &container.id, &network.Id).await;

            docker
                .remove_container(&container.id, None, None, None)
                .await
                .unwrap();

            docker.remove_network(&network.Id).await.unwrap();
        }
    }

    async fn test_image_api(docker: &Docker, name: &str, tag: &str) {
        let mut filter = ContainerFilters::new();
        filter.name("test_container_");
        let containers = docker
            .list_containers(Some(true), None, Some(true), filter.clone())
            .await
            .unwrap();
        assert!(
            containers.is_empty(),
            "remove containers 'test_container_*'"
        );
        test_container(docker, &format!("{name}:{tag}")).await;
        let containers = docker
            .list_containers(Some(true), None, Some(true), filter)
            .await
            .unwrap();
        assert!(containers.is_empty());
    }

    async fn test_image(docker: &Docker, name: &str, tag: &str) {
        let mut src = docker.create_image(name, tag).await.unwrap();
        use futures::stream::StreamExt;
        while let Some(st) = src.next().await.transpose().unwrap() {
            println!("{:?}", st);
        }

        let image = format!("{name}:{tag}");
        let image_file = format!("dockworker_test_{name}_{tag}.tar");

        {
            let res = docker.export_image(&image).await.unwrap();
            let buf = read_bytes_stream_to_end(res).await;
            tokio::fs::write(&image_file, &buf).await.unwrap();
        }

        docker.remove_image(&image, None, None).await.unwrap();
        docker
            .load_image(false, Path::new(&image_file))
            .await
            .unwrap();
        tokio::fs::remove_file(&image_file).await.unwrap();

        test_image_api(docker, name, tag).await;

        docker
            .remove_image(&format!("{name}:{tag}"), None, None)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_api() {
        let docker = Docker::connect_with_defaults().unwrap();

        let (name, tag) = ("alpine", "3.9");
        test_image(&docker, name, tag).await;
    }

    #[cfg(feature = "experimental")]
    #[tokio::test]
    async fn test_container_checkpointing() {
        let docker = Docker::connect_with_defaults().unwrap();
        let (name, tag) = ("alpine", "3.10");
        with_image(&docker, name, tag, |name, tag| {
            let mut create = ContainerCreateOptions::new(&format!("{}:{}", name, tag));
            create.host_config(ContainerHostConfig::new());
            create.cmd("sleep".to_string());
            create.cmd("10000".to_string());
            let container = docker
                .create_container(Some("dockworker_checkpoint_test"), &create)
                .await
                .unwrap();
            docker.start_container(&container.id).await.unwrap();

            docker
                .checkpoint_container(
                    &container.id,
                    &CheckpointCreateOptions {
                        checkpoint_id: "v1".to_string(),
                        checkpoint_dir: None,
                        exit: Some(true),
                    },
                )
                .await
                .unwrap();
            let checkpoints = docker
                .list_container_checkpoints(&container.id, None)
                .await
                .unwrap();
            assert_eq!("v1", &checkpoints[0].Name);

            thread::sleep(Duration::from_secs(1));

            docker
                .resume_container_from_checkpoint(&container.id, "v1", None)
                .await
                .unwrap();

            docker
                .stop_container(&container.id, Duration::new(0, 0))
                .await
                .unwrap();

            docker
                .delete_checkpoint(
                    &container.id,
                    &CheckpointDeleteOptions {
                        checkpoint_id: "v1".to_string(),
                        checkpoint_dir: None,
                    },
                )
                .await
                .unwrap();

            docker
                .remove_container("dockworker_checkpoint_test", None, None, None)
                .await
                .unwrap();
        })
    }

    // generate a file on path which is constructed from size chars alphanum seq
    async fn gen_rand_file(path: &Path, size: usize) -> std::io::Result<()> {
        let mut rng = rand::thread_rng();
        let mut file = tokio::fs::File::create(path).await?;
        let vec: String = iter::repeat(())
            .map(|_| rng.sample(rand::distributions::Alphanumeric) as char)
            .take(size)
            .collect();
        use tokio::io::AsyncWriteExt;
        file.write_all(vec.as_bytes()).await
    }

    async fn equal_file(patha: &Path, pathb: &Path) -> bool {
        let mut filea = tokio::fs::File::open(patha).await.unwrap();
        let mut fileb = tokio::fs::File::open(pathb).await.unwrap();
        let mut a = vec![];
        let mut b = vec![];
        use tokio::io::AsyncReadExt;
        filea.read_to_end(&mut a).await.unwrap();
        fileb.read_to_end(&mut b).await.unwrap();
        a == b
    }

    #[tokio::test]
    async fn test_networks() {
        let docker = Docker::connect_with_defaults().unwrap();
        inspect_networks(&docker).await;
        prune_networks(&docker).await;
    }

    async fn inspect_networks(docker: &Docker) {
        for network in &docker
            .list_networks(ListNetworkFilters::default())
            .await
            .unwrap()
        {
            let network = docker
                .inspect_network(&network.Id, Some(true), None)
                .await
                .unwrap();
            println!("network: {network:?}");
        }
        let create = NetworkCreateOptions::new("dockworker_test_network");
        let res = docker.create_network(&create).await.unwrap();
        let mut filter = ListNetworkFilters::default();
        filter.id(res.Id.as_str().into());
        let networks = docker.list_networks(filter.clone()).await.unwrap();
        assert_eq!(networks.iter().filter(|n| n.Id == res.Id).count(), 1);
        docker.remove_network(&res.Id).await.unwrap();
        let networks = docker.list_networks(filter).await.unwrap();
        assert!(!networks.iter().any(|n| n.Id == res.Id));
    }

    async fn prune_networks(docker: &Docker) {
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
                .await
                .unwrap();

            tokio::time::sleep(Duration::from_secs(1)).await; // drift timestamp in sec
            if i == 3 {
                create_nw_3 = Local::now();
            }
        }

        println!("filter network by label");
        {
            let mut filter = Prune::default();
            filter.label(F::with(&[("test-network-1", None)]));
            let res = docker.prune_networks(filter).await.unwrap();

            assert_eq!(&res.networks_deleted, &["nw_test_1".to_owned()]);
        }
        println!("filter network by negated label");
        {
            let mut filter = Prune::default();
            filter.label_not(F::with(&[("not2", Some("false"))]));
            let res = docker.prune_networks(filter).await.unwrap();
            assert_eq!(&res.networks_deleted, &["nw_test_2".to_owned()]);
        }
        println!("filter network by timestamp");
        {
            let mut filter = Prune::default();
            filter.until(vec![create_nw_3.timestamp()]);
            let res = docker.prune_networks(filter).await.unwrap();
            assert_eq!(res.networks_deleted, &["nw_test_3".to_owned()]);
        }
        println!("filter network by label");
        {
            let mut filter = Prune::default();
            filter.label(F::with(&[("test-network-4", Some("4"))]));
            let res = docker.prune_networks(filter).await.unwrap();
            assert_eq!(&res.networks_deleted, &["nw_test_4".to_owned()]);
        }
        println!("filter network by negated label");
        {
            let mut filter = Prune::default();
            filter.label_not(F::with(&[("alias", Some("my-test-network-6"))]));
            let res = docker.prune_networks(filter).await.unwrap();
            assert_eq!(&res.networks_deleted, &["nw_test_5".to_owned()]);
        }
        println!("prune network");
        {
            let res = docker.prune_networks(Prune::default()).await.unwrap();
            assert_eq!(&res.networks_deleted, &["nw_test_6".to_owned()]);
        }
    }

    /// This is executed after `docker-compose build iostream`
    #[tokio::test]
    #[ignore]
    async fn attach_container() {
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
            .await
            .unwrap();

        docker.start_container(&container.id).await.unwrap();

        let res = docker
            .attach_container(&container.id, None, true, true, false, true, true)
            .await
            .unwrap();

        let kill = async {
            // wait a moment
            tokio::time::sleep(std::time::Duration::from_secs(3)).await;
            // We've successfully attached, tell the container
            // to continue printing to stdout and stderr
            docker
                .kill_container(&container.id, Signal::from(SIGUSR1))
                .await
                .unwrap();
        };
        let (ret, _) = futures::future::join(read_frame_all(res), kill).await;
        let (_stdin_buf, stdout_buf, stderr_buf) = ret.unwrap();

        // expected files
        let exp_stdout_buf = read_file(root.join(exps[0])).await;
        let exp_stderr_buf = read_file(root.join(exps[1])).await;
        assert_eq!(exp_stdout_buf, stdout_buf);
        assert_eq!(exp_stderr_buf, stderr_buf);

        docker.wait_container(&container.id).await.unwrap();
        docker
            .remove_container(&container.id, None, None, None)
            .await
            .unwrap();
    }

    /// This is executed after `docker-compose build iostream`
    #[tokio::test]
    #[ignore]
    async fn exec_container() {
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
            .await
            .unwrap();
        docker.start_container(&container.id).await.unwrap();

        let mut exec_config = CreateExecOptions::new();
        exec_config
            .cmd("./entrypoint.sh".to_owned())
            .cmd(exps[0].to_owned())
            .cmd(exps[1].to_owned());

        let exec_instance = docker
            .exec_container(&container.id, &exec_config)
            .await
            .unwrap();
        let exec_start_config = StartExecOptions::new();
        let res = docker
            .start_exec(&exec_instance.id, &exec_start_config)
            .await
            .unwrap();

        let (_stdin_buf, stdout_buf, stderr_buf) = read_frame_all(res).await.unwrap();

        // expected files
        let exp_stdout_buf = read_file(root.join(exps[0])).await;
        let exp_stderr_buf = read_file(root.join(exps[1])).await;

        assert_eq!(exp_stdout_buf, stdout_buf);
        assert_eq!(exp_stderr_buf, stderr_buf);

        let exec_inspect = docker.exec_inspect(&exec_instance.id).await.unwrap();

        assert_eq!(exec_inspect.ExitCode, Some(0));
        assert_eq!(exec_inspect.Running, false);

        docker.wait_container(&container.id).await.unwrap();
        docker
            .remove_container(&container.id, None, None, None)
            .await
            .unwrap();
    }

    /// This is executed after `docker-compose build signal`
    #[tokio::test]
    #[ignore]
    async fn signal_container() {
        use crate::signal::*;
        let docker = Docker::connect_with_defaults().unwrap();

        let image_name = "test-signal:latest";
        let host_config = ContainerHostConfig::new();
        let mut create = ContainerCreateOptions::new(image_name);
        create.host_config(host_config);

        let container = docker
            .create_container(Some("signal_container_test"), &create)
            .await
            .unwrap();
        docker.start_container(&container.id).await.unwrap();
        let res = docker
            .attach_container(&container.id, None, true, true, false, true, true)
            .await
            .unwrap();
        let signals = [SIGHUP, SIGINT, SIGUSR1, SIGUSR2, SIGTERM];
        let signalstrs = vec![
            "HUP".to_string(),
            "INT".to_string(),
            "USR1".to_string(),
            "USR2".to_string(),
            "TERM".to_string(),
        ];
        let kill = async {
            // wait a moment
            tokio::time::sleep(std::time::Duration::from_secs(3)).await;
            for sig in signals {
                trace!("cause signal: {:?}", sig);
                docker
                    .kill_container(&container.id, Signal::from(sig))
                    .await
                    .unwrap();
            }
        };
        let (ret, _) = futures::future::join(read_frame_all(res), kill).await;
        let (_stdin_buf, stdout_buf, _stderr_buf) = ret.unwrap();

        let stdout = std::io::Cursor::new(stdout_buf);
        let stdout_buffer = std::io::BufReader::new(stdout);
        use std::io::BufRead;
        let lines = stdout_buffer.lines().map(|line| line.unwrap());
        assert!(lines.eq(signalstrs));

        trace!("wait");
        assert_eq!(
            docker.wait_container(&container.id).await.unwrap(),
            ExitStatus::new(15)
        );

        trace!("remove container");
        docker
            .remove_container(&container.id, None, None, None)
            .await
            .unwrap();
    }
}
