use std::env;
use std::ffi::OsStr;
use std::fmt;
use std::fs::File;
use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::result;
use std::time::Duration;
use url;

use container::{
    AttachResponse, Container, ContainerFilters, ContainerInfo, ExecInfo, ExitStatus, LogResponse,
};
pub use credentials::{Credential, UserPassword};
use errors::*;
use filesystem::FilesystemChange;
use hyper_client::HyperClient;
use image::{Image, ImageId, SummaryImage};
use options::*;
use process::{Process, Top};
use stats::StatsReader;
use system::{AuthToken, SystemInfo};
use tar::{self, Archive};
use version::Version;

use header::XRegistryAuth;
use http_client::{HaveHttpClient, HttpClient};
use hyper_client::{ContentType, Headers, Response, StatusCode};
use mime;
use response::Response as DockerResponse;
use serde::de::DeserializeOwned;
use serde_json;
use signal::Signal;

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
        let home = dirs::home_dir().ok_or_else(|| ErrorKind::NoCertPath)?;
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
    protocol: Protocol,
    /// http headers used for any requests
    headers: Headers,
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
    if res.status == StatusCode::NO_CONTENT {
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
            headers: Headers::new(),
            credential: None,
        }
    }

    pub fn set_credential(&mut self, credential: Credential) {
        self.credential = Some(credential)
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
                )
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
        if addr.starts_with("unix://") {
            let client = HyperClient::connect_with_unix(&addr[7..]);
            Ok(Docker::new(client, Protocol::Unix))
        } else {
            let client = HyperClient::connect_with_unix(addr);
            Ok(Docker::new(client, Protocol::Unix))
        }
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

    #[deprecated]
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
            None => format!("/containers/create"),
        };

        let json_body = serde_json::to_string(&option)?;
        let mut headers = self.headers().clone();
        headers.set::<ContentType>(ContentType::json());
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
            .post(self.headers(), &format!("/containers/{}/start", id), "")
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
            .and_then(no_content)
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
        headers.set::<ContentType>(ContentType::json());
        self.http_client()
            .post(&headers, &format!("/containers/{}/exec", id), &json_body)
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
        headers.set::<ContentType>(ContentType::json());

        self.http_client()
            .post(&headers, &format!("/exec/{}/start", id), &json_body)
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
            .get(self.headers(), &format!("/exec/{}/json", id))
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
    pub fn container_top(&self, container: &Container) -> Result<Top> {
        self.http_client()
            .get(self.headers(), &format!("/containers/{}/top", container.Id))
            .and_then(api_result)
    }

    pub fn processes(&self, container: &Container) -> Result<Vec<Process>> {
        let top = self.container_top(container)?;
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
    /// /containers/{id}/stats
    pub fn stats(&self, container: &Container) -> Result<StatsReader> {
        let res = self.http_client().get(
            self.headers(),
            &format!("/containers/{}/stats", container.Id),
        )?;
        Ok(StatsReader::new(res))
    }

    /// Wait for a container
    ///
    /// # API
    /// /containers/{id}/wait
    pub fn wait_container(&self, id: &str) -> Result<ExitStatus> {
        self.http_client()
            .post(self.headers(), &format!("/containers/{}/wait", id), "")
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
    pub fn get_file(&self, id: &str, path: &Path) -> Result<tar::Archive<Box<Read>>> {
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
                    Ok(tar::Archive::new(Box::new(res) as Box<Read>))
                } else {
                    Err(serde_json::from_reader::<_, DockerError>(res)?.into())
                }
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
        let application_tar: mime::Mime = "application/x-tar".parse()?;
        headers.set::<ContentType>(ContentType(application_tar));
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
    ) -> Result<Box<Iterator<Item = Result<DockerResponse>>>> {
        let mut param = url::form_urlencoded::Serializer::new(String::new());
        param.append_pair("fromImage", image);
        param.append_pair("tag", tag);

        let mut headers = self.headers().clone();
        if let Some(ref credential) = self.credential {
            headers.set::<XRegistryAuth>(credential.clone().into());
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
            .get(self.headers(), &format!("/images/{}/json", name))
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
            headers.set::<XRegistryAuth>(credential.clone().into());
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
            &format!(
                r#"{{ "dangling": {{ "{}": true }} }}"#,
                dangling.to_string()
            ),
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
            .get(self.headers(), &format!("/images/{}/history", name))
            .and_then(api_result)
            .map(|mut hs: Vec<ImageLayer>| {
                hs.iter_mut().for_each(|change| {
                    if change.id == Some("<missing>".into()) {
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
    pub fn export_image(&self, name: &str) -> Result<Box<Read>> {
        self.http_client()
            .get(self.headers(), &format!("/images/{}/get", name))
            .and_then(|res| {
                if res.status.is_success() {
                    Ok(Box::new(res) as Box<Read>)
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
        let application_tar: mime::Mime = "application/x-tar".parse()?;
        headers.set::<ContentType>(ContentType(application_tar));
        let res = self.http_client().post_file(
            &headers,
            &format!("/images/load?quiet={}", quiet),
            path,
        )?;
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
                let id = stem
                    .to_str()
                    .ok_or(ErrorKind::Unknown(format!("convert to String: {:?}", stem)))?;
                return Ok(ImageId::new(id.to_string()));
            }
        }
        Err(ErrorKind::Unknown("no expected file: XXXXXX.json".to_owned()).into())
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
        headers.set(ContentType::json());
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
    pub fn container_info(&self, container: &Container) -> Result<ContainerInfo> {
        self.http_client()
            .get(
                self.headers(),
                &format!("/containers/{}/json", container.Id),
            )
            .and_then(api_result)
    }

    /// Get changes on a container's filesystem.
    ///
    /// (This is the same as `docker container diff` command.)
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

    /// Get monitor events
    ///
    /// # API
    /// /events
    pub fn events(
        &self,
        since: Option<u64>,
        until: Option<u64>,
        filters: Option<EventFilters>,
    ) -> Result<Box<Iterator<Item = Result<EventResponse>>>> {
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
            .and_then(|res| {
                Ok(Box::new(
                    serde_json::Deserializer::from_reader(res)
                        .into_iter::<EventResponse>()
                        .map(|event_response| Ok(event_response?)),
                )
                    as Box<Iterator<Item = Result<EventResponse>>>)
            })
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
    extern crate rand;

    use super::*;
    use std::convert::From;
    use std::env;
    use std::fs::{remove_file, File};
    use std::io::{self, Read, Write};
    use std::iter::{self, Iterator};
    use std::path::PathBuf;

    use self::rand::Rng;
    use tar::Builder as TarBuilder;

    use container;

    #[test]
    fn test_server_access() {
        let docker = Docker::connect_with_defaults().unwrap();
        assert!(docker.ping().is_ok());
    }

    #[test]
    fn test_info() {
        let docker = Docker::connect_with_defaults().unwrap();
        assert!(docker.system_info().is_ok());
    }

    #[test]
    fn get_version() {
        let docker = Docker::connect_with_defaults().unwrap();
        assert!(docker.version().is_ok());
    }

    #[test]
    fn get_events() {
        let docker = Docker::connect_with_defaults().unwrap();
        assert!(docker.events(None, None, None).is_ok());
    }

    #[test]
    fn create_remove_image() {
        let docker = Docker::connect_with_defaults().unwrap();
        let (name, tag) = ("debian", "latest");
        let sts = docker
            .create_image(name, tag)
            .map(|sts| sts.for_each(|st| assert!(st.is_ok())));
        assert!(sts.is_ok());
        assert!(docker
            .remove_image(&format!("{}:{}", name, tag), None, None)
            .is_ok());
    }

    #[test]
    fn create_remove_container() {
        let docker = Docker::connect_with_defaults().unwrap();
        let (name, tag) = ("hello-world", "linux");
        assert!(docker
            .create_image(name, tag)
            .map(|sts| sts.for_each(|st| println!("{:?}", st)))
            .is_ok());
        let mut create = ContainerCreateOptions::new(&format!("{}:{}", name, tag));
        create.host_config(ContainerHostConfig::new());

        assert!(docker
            .create_container(Some("dockworker_test"), &create)
            .is_ok());
        assert!(docker
            .remove_container("dockworker_test", None, None, None)
            .is_ok());
        assert!(docker
            .remove_image(&format!("{}:{}", name, tag), None, None)
            .is_ok());
    }

    #[test]
    fn auto_remove_container() {
        let docker = Docker::connect_with_defaults().unwrap();
        let (name, tag) = ("alpine", "3.7");
        assert!(docker
            .create_image(name, tag)
            .map(|sts| sts.for_each(|st| println!("{:?}", st)))
            .is_ok());
        let mut host_config = ContainerHostConfig::new();
        host_config.auto_remove(true);
        let mut create = ContainerCreateOptions::new(&format!("{}:{}", name, tag));
        create.host_config(host_config);

        let container = docker
            .create_container(Some("dockworker_auto_remove_container"), &create)
            .unwrap();
        assert!(docker.start_container(&container.id).is_ok());
        assert!(docker.wait_container(&container.id).is_ok());
        assert!(
            docker
                .remove_container("dockworker_auto_remove_container", None, None, None)
                .is_err() // 'no such container' or 'removel container in progress'
        );
        assert!(docker
            .remove_image(&format!("{}:{}", name, tag), Some(true), None)
            .is_ok());
    }

    fn pull_image(docker: &Docker, name: &str, tag: &str) {
        assert!(docker
            .create_image(name, tag)
            .map(|sts| sts.for_each(|st| println!("{:?}", st)))
            .is_ok());
    }

    #[test]
    fn export_load_image() {
        let docker = Docker::connect_with_defaults().unwrap();
        pull_image(&docker, "alpine", "latest");

        {
            let mut file = File::create("dockworker_test_alpine.tar").unwrap();
            let mut res = docker.export_image("alpine:latest").unwrap();
            io::copy(&mut res, &mut file).unwrap();
        }

        assert!(docker.remove_image("alpine:latest", None, None).is_ok());
        assert!(docker
            .load_image(false, Path::new("dockworker_test_alpine.tar"))
            .is_ok());
        assert!(remove_file("dockworker_test_alpine.tar").is_ok());
    }

    fn with_image<F>(docker: &Docker, name: &str, tag: &str, f: F)
    where
        F: Fn(&str, &str),
    {
        pull_image(&docker, name, tag);
        f(name, tag);
        assert!(docker
            .remove_image(&format!("{}:{}", name, tag), None, None)
            .is_ok());
    }

    #[test]
    fn wait_container() {
        let docker = Docker::connect_with_defaults().unwrap();
        let (name, tag) = ("alpine", "3.4");
        let container_name = "alpine34_exit0";
        with_image(&docker, name, tag, |name, tag| {
            let mut create = ContainerCreateOptions::new(&format!("{}:{}", name, tag));
            create.cmd("ls".to_string());
            assert!(docker
                .create_container(Some(container_name), &create)
                .is_ok());
            assert_eq!(
                docker.wait_container(container_name).unwrap(),
                ExitStatus::new(0)
            );
            assert!(docker
                .remove_container(container_name, None, None, None)
                .is_ok());
        })
    }

    // generate a file on path which is constructed from size chars alphanum seq
    fn gen_rand_file(path: &Path, size: usize) -> io::Result<()> {
        let mut rng = rand::thread_rng();
        let mut file = File::create(path)?;
        let vec: String = iter::repeat(())
            .map(|_| rng.sample(rand::distributions::Alphanumeric))
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
    fn put_file_to_container() {
        let docker = Docker::connect_with_defaults().unwrap();
        let (name, tag) = ("alpine", "3.6");

        let temp_dir = env::temp_dir();
        let test_file = &temp_dir.join("test_file");

        with_image(&docker, name, tag, |name, tag| {
            let create = ContainerCreateOptions::new(&format!("{}:{}", name, tag));
            let container = docker.create_container(None, &create).unwrap();
            assert!(docker.start_container(&container.id).is_ok());

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

            assert!(match docker.get_file(&container.id, test_file) {
                Ok(_) => false,
                Err(Error(ErrorKind::Docker(_), _)) => true, // not found
                Err(_) => false,
            });

            docker
                .put_file(
                    &container.id,
                    &test_file.with_extension("tar"),
                    Path::new("/"),
                    true,
                )
                .unwrap();

            docker
                .get_file(&container.id, test_file)
                .unwrap()
                .unpack(temp_dir.join("put"))
                .unwrap();

            docker.wait_container(&container.id).unwrap();

            docker
                .remove_container(&container.id, None, None, None)
                .unwrap();
        });

        assert!(equal_file(
            test_file,
            &temp_dir.join("put").join(test_file.file_name().unwrap())
        ));
    }

    #[test]
    fn log_container() {
        let docker = Docker::connect_with_defaults().unwrap();
        let (name, tag) = ("alpine", "3.5");
        docker
            .create_image(name, tag)
            .map(|sts| sts.for_each(|st| println!("{:?}", st)))
            .unwrap();
        let mut create_options = ContainerCreateOptions::new(&format!("{}:{}", name, tag));
        create_options.entrypoint(vec!["cat".into()]);
        create_options.cmd("/etc/motd".to_string());

        let log_options = ContainerLogOptions {
            stdout: true,
            stderr: true,
            follow: true,
            ..ContainerLogOptions::default()
        };

        let lines = {
            let container = docker.create_container(None, &create_options).unwrap();
            docker.start_container(&container.id).unwrap();
            let log = docker.log_container(&container.id, &log_options).unwrap();
            let lines = BufReader::new(log)
                .lines()
                .map(|x| x.unwrap())
                .fold("".to_string(), |acc, s| acc + &s);
            docker
                .remove_container(&container.id, None, None, None)
                .unwrap();
            lines
        };

        let once = {
            let container = docker.create_container(None, &create_options).unwrap();
            docker.start_container(&container.id).unwrap();
            let mut log = docker.log_container(&container.id, &log_options).unwrap();
            let once = log
                .output()
                .unwrap()
                .replace(|c| c == '\r' || c == '\n', "")
                .to_owned();
            docker
                .remove_container(&container.id, None, None, None)
                .unwrap();
            once
        };

        assert_eq!(lines, once);

        docker
            .remove_image(&format!("{}:{}", name, tag), None, None)
            .unwrap();
    }

    /// This is executed after `docker-compose build iostream`
    #[test]
    #[ignore]
    fn attach_container() {
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
            .host_config(host_config);

        let container = docker.create_container(None, &create).unwrap();
        docker.start_container(&container.id).unwrap();
        let res = docker
            .attach_container(&container.id, None, true, true, false, true, true)
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

        docker.wait_container(&container.id).unwrap();
        docker
            .remove_container(&container.id, None, None, None)
            .unwrap();
    }

    /// This is executed after `docker-compose build iostream-exec`
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

        let container = docker.create_container(None, &create).unwrap();
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
        use nix::sys::signal::Signal::*;
        let docker = Docker::connect_with_defaults().unwrap();

        let image_name = "test-signal:latest";
        let host_config = ContainerHostConfig::new();
        let mut create = ContainerCreateOptions::new(image_name);
        create.host_config(host_config);

        let container = docker.create_container(None, &create).unwrap();
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
                .kill_container(&container.id, Signal::from(sig.clone()))
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
}
