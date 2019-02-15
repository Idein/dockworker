use byteorder::{BigEndian, ReadBytesExt};
use errors::Result;
use hyper_client::Response;
use std;
use std::cell::{Ref, RefCell, RefMut};
use std::collections::HashMap;
use std::io::{self, Read};
use std::rc::Rc;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct Container {
    pub Id: String,
    pub Image: String,
    pub Status: String,
    pub Command: String,
    pub Created: u64,
    pub Names: Vec<String>,
    pub Ports: Vec<Port>,
    pub SizeRw: Option<u64>, // I guess it is optional on Mac.
    pub SizeRootFs: Option<u64>,
    pub Labels: Option<HashMap<String, String>>,
    pub HostConfig: HostConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct Port {
    pub IP: Option<String>,
    pub PrivatePort: u64,
    pub PublicPort: Option<u64>,
    pub Type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct HostConfig {
    pub NetworkMode: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct ContainerInfo {
    pub AppArmorProfile: String,
    pub Args: Vec<String>,
    pub Config: Config,
    pub Created: String,
    pub Driver: String,
    // ExecIDs
    // GraphDriver
    // HostConfig
    pub HostnamePath: String,
    pub HostsPath: String,
    pub Id: String,
    pub Image: String,
    pub LogPath: String,
    pub MountLabel: String,
    pub Mounts: Vec<Mount>,
    pub Name: String,
    pub NetworkSettings: NetworkSettings,
    pub Path: String,
    pub ProcessLabel: String,
    pub ResolvConfPath: String,
    pub RestartCount: u64,
    pub State: State,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(non_snake_case)]
pub struct ExecProcessConfig {
    pub arguments: Vec<String>,
    pub entrypoint: String,
    pub privileged: bool,
    pub tty: bool,
    pub user: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(non_snake_case)]
pub struct ExecInfo {
    pub CanRemove: bool,
    pub ContainerID: String,
    pub DetachKeys: String,
    pub ExitCode: Option<u32>,
    pub ID: String,
    pub OpenStderr: bool,
    pub OpenStdin: bool,
    pub OpenStdout: bool,
    pub ProcessConfig: ExecProcessConfig,
    pub Running: bool,
    pub Pid: u64,
}

/// This type represents a `struct{}` in the Go code.
pub type UnspecifiedObject = HashMap<String, String>;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct Config {
    pub AttachStderr: bool,
    pub AttachStdin: bool,
    pub AttachStdout: bool,
    // TODO: Verify that this is never just a `String`.
    //pub Cmd: Vec<String>,
    pub Domainname: String,
    // TODO: The source says `Option<String>` but I've seen
    // `Option<Vec<String>>` on the wire.  Ignore until we figure it out.
    //pub Entrypoint: Option<Vec<String>>,
    pub Env: Option<Vec<String>>,
    pub ExposedPorts: Option<HashMap<String, UnspecifiedObject>>,
    pub Hostname: String,
    pub Image: String,
    pub Labels: HashMap<String, String>,
    // TODO: We don't know exacly what this vec contains.
    //pub OnBuild: Option<Vec<???>>,
    pub OpenStdin: bool,
    pub StdinOnce: bool,
    pub Tty: bool,
    pub User: String,
    pub Volumes: Option<HashMap<String, UnspecifiedObject>>,
    pub WorkingDir: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct Mount {
    // Name (optional)
    // Driver (optional)
    pub Source: String,
    pub Destination: String,
    pub Mode: String,
    pub RW: bool,
    pub Propagation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct NetworkSettings {
    pub Bridge: String,
    pub EndpointID: String,
    pub Gateway: String,
    pub GlobalIPv6Address: String,
    pub GlobalIPv6PrefixLen: u32,
    pub HairpinMode: bool,
    pub IPAddress: String,
    pub IPPrefixLen: u32,
    pub IPv6Gateway: String,
    pub LinkLocalIPv6Address: String,
    pub LinkLocalIPv6PrefixLen: u32,
    pub MacAddress: String,
    pub Networks: HashMap<String, Network>,
    pub Ports: Option<HashMap<String, Option<Vec<PortMapping>>>>,
    pub SandboxID: String,
    pub SandboxKey: String,
    // These two are null in the current output.
    //pub SecondaryIPAddresses: ,
    //pub SecondaryIPv6Addresses: ,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct Network {
    pub Aliases: Option<Vec<String>>,
    pub EndpointID: String,
    pub Gateway: String,
    pub GlobalIPv6Address: String,
    pub GlobalIPv6PrefixLen: u32,
    //pub IPAMConfig: ,
    pub IPAddress: String,
    pub IPPrefixLen: u32,
    pub IPv6Gateway: String,
    //pub Links:
    pub MacAddress: String,
    pub NetworkID: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct PortMapping {
    pub HostIp: String,
    pub HostPort: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct State {
    pub Status: String,
    pub Running: bool,
    pub Paused: bool,
    pub Restarting: bool,
    pub OOMKilled: bool,
    pub Dead: bool,
    // I don't know whether PIDs can be negative here.  They're normally
    // positive, but sometimes negative PIDs are used in certain APIs.
    pub Pid: i64,
    pub ExitCode: i64,
    pub Error: String,
    pub StartedAt: String,
    pub FinishedAt: String,
}

impl std::fmt::Display for Container {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::result::Result<(), std::fmt::Error> {
        write!(f, "{}", self.Id)
    }
}

impl std::fmt::Display for ContainerInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::result::Result<(), std::fmt::Error> {
        write!(f, "{}", self.Id)
    }
}

#[derive(Debug, PartialEq, PartialOrd, Serialize)]
pub struct ContainerFilters {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    id: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    name: Vec<String>,
}

impl Default for ContainerFilters {
    fn default() -> Self {
        Self {
            id: vec![],
            name: vec![],
        }
    }
}

impl ContainerFilters {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn id(&mut self, id: &str) -> &mut Self {
        self.id.push(id.to_owned());
        self
    }

    pub fn name(&mut self, name: &str) -> &mut Self {
        self.name.push(name.to_owned());
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum ContainerStdioType {
    Stdin,
    Stdout,
    Stderr,
}

/// response fragment of the attach container api
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct AttachResponseFrame {
    type_: ContainerStdioType,
    frame: Vec<u8>,
}

impl AttachResponseFrame {
    fn new(type_: ContainerStdioType, frame: Vec<u8>) -> Self {
        Self { type_, frame }
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.frame
    }
}

#[derive(Debug, Clone)]
struct ContainerStdio {
    /// io type
    type_: ContainerStdioType,
    /// shared source (response)
    src: Rc<RefCell<AttachResponseIter>>,
    stdin_buff: Rc<RefCell<Vec<u8>>>,
    stdout_buff: Rc<RefCell<Vec<u8>>>,
    stderr_buff: Rc<RefCell<Vec<u8>>>,
}

#[derive(Debug, Clone)]
pub struct ContainerStdin {
    body: ContainerStdio,
}

#[derive(Debug, Clone)]
pub struct ContainerStdout {
    body: ContainerStdio,
}

#[derive(Debug, Clone)]
pub struct ContainerStderr {
    body: ContainerStdio,
}

impl ContainerStdin {
    fn new(body: ContainerStdio) -> Self {
        Self { body }
    }
}

impl ContainerStdout {
    fn new(body: ContainerStdio) -> Self {
        Self { body }
    }
}

impl ContainerStderr {
    fn new(body: ContainerStdio) -> Self {
        Self { body }
    }
}

impl Read for ContainerStdin {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.body.read(buf)
    }
}

impl Read for ContainerStdout {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.body.read(buf)
    }
}

impl Read for ContainerStderr {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.body.read(buf)
    }
}

#[derive(Debug)]
pub struct AttachContainer {
    pub stdin: ContainerStdin,
    pub stdout: ContainerStdout,
    pub stderr: ContainerStderr,
}

impl AttachContainer {
    fn new(stdin: ContainerStdin, stdout: ContainerStdout, stderr: ContainerStderr) -> Self {
        Self {
            stdin,
            stdout,
            stderr,
        }
    }
}

impl ContainerStdio {
    fn new(
        type_: ContainerStdioType,
        src: Rc<RefCell<AttachResponseIter>>,
        stdin_buff: Rc<RefCell<Vec<u8>>>,
        stdout_buff: Rc<RefCell<Vec<u8>>>,
        stderr_buff: Rc<RefCell<Vec<u8>>>,
    ) -> Self {
        Self {
            type_,
            src,
            stdin_buff,
            stdout_buff,
            stderr_buff,
        }
    }

    fn forcused_buff(&self) -> Ref<Vec<u8>> {
        use container::ContainerStdioType::*;
        match self.type_ {
            Stdin => self.stdin_buff.borrow(),
            Stdout => self.stdout_buff.borrow(),
            Stderr => self.stderr_buff.borrow(),
        }
    }

    fn forcused_buff_mut(&self) -> RefMut<Vec<u8>> {
        use container::ContainerStdioType::*;
        match self.type_ {
            Stdin => self.stdin_buff.borrow_mut(),
            Stdout => self.stdout_buff.borrow_mut(),
            Stderr => self.stderr_buff.borrow_mut(),
        }
    }

    // read next chunk from response to the inner buffer
    fn readin_next(&mut self) -> io::Result<usize> {
        use container::ContainerStdioType::*;

        while let Some(xs) = self.src.borrow_mut().next() {
            let AttachResponseFrame { type_, mut frame } = xs?;
            let len = frame.len();
            match type_ {
                Stdin => self.stdin_buff.borrow_mut().append(&mut frame),
                Stdout => self.stdout_buff.borrow_mut().append(&mut frame),
                Stderr => self.stderr_buff.borrow_mut().append(&mut frame),
            }
            if type_ == self.type_ {
                return Ok(len);
            }
        }

        Ok(0) // end of stream
    }
}

impl Read for ContainerStdio {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if self.forcused_buff().len() == 0 {
            match self.readin_next() {
                Ok(0) => return Ok(0),
                Err(e) => return Err(e),
                _ => {}
            }
        }
        let inner_buf_len = self.forcused_buff().len(); // > 0

        if inner_buf_len <= buf.len() {
            debug!("{} <= {}", inner_buf_len, buf.len());
            buf[..inner_buf_len].copy_from_slice(&self.forcused_buff()); // copy
            self.forcused_buff_mut().clear(); // clear inner buffer
            Ok(inner_buf_len)
        } else {
            // inner_buf_len > buf.len()
            debug!("{} > {}", inner_buf_len, buf.len());
            let buf_len = buf.len();
            buf.copy_from_slice(&self.forcused_buff()[..buf_len]); // copy (fill buf)
            let mut inner_buf = self.forcused_buff_mut();
            inner_buf.drain(..buf_len); // delete _size_ elementes from the head of buf
            Ok(buf_len)
        }
    }
}

/// Response of attach to container api
#[derive(Debug)]
pub struct AttachResponse {
    res: Response,
}

impl AttachResponse {
    pub fn new(res: Response) -> Self {
        Self { res }
    }
}

impl From<AttachResponse> for AttachContainer {
    fn from(res: AttachResponse) -> Self {
        let iter = Rc::new(RefCell::new(res.res.into())); // into_iter
        let stdin_buff = Rc::new(RefCell::new(Vec::new()));
        let stdout_buff = Rc::new(RefCell::new(Vec::new()));
        let stderr_buff = Rc::new(RefCell::new(Vec::new()));
        let stdin = ContainerStdin::new(ContainerStdio::new(
            ContainerStdioType::Stdin,
            Rc::clone(&iter),
            Rc::clone(&stdin_buff),
            Rc::clone(&stdout_buff),
            Rc::clone(&stderr_buff),
        ));
        let stdout = ContainerStdout::new(ContainerStdio::new(
            ContainerStdioType::Stdout,
            Rc::clone(&iter),
            Rc::clone(&stdin_buff),
            Rc::clone(&stdout_buff),
            Rc::clone(&stderr_buff),
        ));
        let stderr = ContainerStderr::new(ContainerStdio::new(
            ContainerStdioType::Stderr,
            Rc::clone(&iter),
            Rc::clone(&stdin_buff),
            Rc::clone(&stdout_buff),
            Rc::clone(&stderr_buff),
        ));
        AttachContainer::new(stdin, stdout, stderr)
    }
}

#[derive(Debug)]
struct AttachResponseIter {
    res: Response,
}

impl AttachResponseIter {
    fn new(res: Response) -> Self {
        Self { res }
    }
}

impl From<Response> for AttachResponseIter {
    fn from(res: Response) -> Self {
        Self::new(res)
    }
}

impl Iterator for AttachResponseIter {
    type Item = io::Result<AttachResponseFrame>;
    fn next(&mut self) -> Option<Self::Item> {
        use container::ContainerStdioType::*;

        let mut buf = [0u8; 8];
        // read header
        if let Err(err) = self.res.read_exact(&mut buf) {
            return if err.kind() == io::ErrorKind::UnexpectedEof {
                None // end of stream
            } else {
                Some(Err(err))
            };
        }
        // read body
        let mut frame_size_raw = &buf[4..];
        let frame_size = frame_size_raw.read_u32::<BigEndian>().unwrap();
        let mut frame = vec![0; frame_size as usize];
        if let Err(io) = self.res.read_exact(&mut frame) {
            return Some(Err(io));
        }
        match buf[0] {
            0 => Some(Ok(AttachResponseFrame::new(Stdin, frame))),
            1 => Some(Ok(AttachResponseFrame::new(Stdout, frame))),
            2 => Some(Ok(AttachResponseFrame::new(Stderr, frame))),
            n => {
                error!("unexpected kind of chunk: {}", n);
                None
            }
        }
    }
}

/// Response of log container api
#[derive(Debug)]
pub struct LogResponse {
    res: Response,
}

impl From<Response> for LogResponse {
    fn from(res: Response) -> Self {
        Self { res }
    }
}

impl Read for LogResponse {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.res.read(buf)
    }
}

impl LogResponse {
    fn new(res: Response) -> Self {
        Self { res }
    }

    pub fn output(&mut self) -> Result<String> {
        let mut str = String::new();
        self.res.read_to_string(&mut str)?;
        Ok(str)
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct ExitStatus {
    StatusCode: i32,
}

impl ExitStatus {
    pub fn new(status_code: i32) -> Self {
        Self {
            StatusCode: status_code,
        }
    }

    pub fn into_inner(self) -> i32 {
        self.StatusCode
    }
}

impl From<i32> for ExitStatus {
    fn from(status_code: i32) -> Self {
        Self::new(status_code)
    }
}
