use byteorder::{BigEndian, ReadBytesExt};
use hyper::client::response::Response;
use std;
use std::collections::HashMap;
use std::io::{self, Read, Write};
use std::rc::Rc;
use std::cell::RefCell;

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

/// This type represents a `struct{}` in the Go code.
pub type UnspecifiedObject = HashMap<String, String>;

#[derive(Debug, Clone, Serialize, Deserialize)]
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

/// fragment of response of attach to container api
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub enum AttachResponseFrame {
    Stdin(Vec<u8>),
    Stdout(Vec<u8>),
    Stderr(Vec<u8>),
}

impl AttachResponseFrame {
    pub fn as_bytes(&self) -> &[u8] {
        use self::AttachResponseFrame::*;
        match self {
            &Stdin(ref vs) => &vs,
            &Stdout(ref vs) => &vs,
            &Stderr(ref vs) => &vs,
        }
    }
}

#[derive(Debug)]
pub struct ContainerStdin {}

#[derive(Debug)]
pub struct ContainerStdout {
    /// shared source (response)
    src: Rc<RefCell<AttachResponseIter>>,
    stdin_buff: Rc<RefCell<Vec<u8>>>,
    stdout_buff: Rc<RefCell<Vec<u8>>>,
    stderr_buff: Rc<RefCell<Vec<u8>>>,
}

#[derive(Debug)]
pub struct ContainerStderr {}

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

impl Read for ContainerStdin {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        unimplemented!("ContainerStdin::Read")
    }
}

impl ContainerStdout {
    fn new(
        src: Rc<RefCell<AttachResponseIter>>,
        stdin_buff: Rc<RefCell<Vec<u8>>>,
        stdout_buff: Rc<RefCell<Vec<u8>>>,
        stderr_buff: Rc<RefCell<Vec<u8>>>,
    ) -> Self {
        Self {
            src,
            stdin_buff,
            stdout_buff,
            stderr_buff,
        }
    }
}

impl Read for ContainerStdout {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        use container::AttachResponseFrame::*;
        while self.stdout_buff.borrow().len() < buf.len() {
            if let Some(xs) = self.src.borrow_mut().next() {
                match xs? {
                    Stdin(mut chunk) => self.stdin_buff.borrow_mut().append(&mut chunk),
                    Stdout(mut chunk) => self.stdout_buff.borrow_mut().append(&mut chunk),
                    Stderr(mut chunk) => self.stderr_buff.borrow_mut().append(&mut chunk),
                }
            } else {
                let size = self.stdout_buff.borrow().len();
                for (i, c) in self.stdout_buff.borrow().iter().enumerate() {
                    buf[i] = *c;
                }
                self.stdout_buff.borrow_mut().clear();
                return Ok(size);
            }
        }
        let size = buf.len();
        for (i, p) in buf.iter_mut().enumerate() {
            let src = self.stdout_buff.borrow()[i];
            *p = src;
        }
        let mut buf = self.stdout_buff.borrow_mut();
        buf.drain(..size);
        Ok(size)
    }
}

impl Read for ContainerStderr {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        unimplemented!("ContainerStdin::Read")
    }
}

pub mod stdio {
    use super::*;

    #[derive(Debug)]
    pub enum Stdio {
        /// connect to Read object
        Piped,
        /// ignore whole contents like /dev/null
        Null,
    }

    impl Stdio {
        pub fn piped() -> Self {
            Stdio::Piped
        }
        pub fn null() -> Self {
            Stdio::Null
        }
    }
}

/// Response of attach to container api
#[derive(Debug)]
pub struct AttachResponse {
    res: Response,
    stdin: stdio::Stdio,
    stdout: stdio::Stdio,
    stderr: stdio::Stdio,
}

impl AttachResponse {
    pub fn new(res: Response) -> Self {
        Self {
            res,
            stdin: stdio::Stdio::piped(),
            stdout: stdio::Stdio::piped(),
            stderr: stdio::Stdio::piped(),
        }
    }

    pub fn stdin(&mut self, stdin: stdio::Stdio) -> &mut Self {
        self.stdin = stdin;
        self
    }

    pub fn stdout(&mut self, stdout: stdio::Stdio) -> &mut Self {
        self.stdout = stdout;
        self
    }

    pub fn stderr(&mut self, stderr: stdio::Stdio) -> &mut Self {
        self.stderr = stderr;
        self
    }
}

impl From<AttachResponse> for AttachContainer {
    fn from(res: AttachResponse) -> Self {
        let iter = Rc::new(RefCell::new(res.res.into())); // into_iter
        let stdin_buff = Rc::new(RefCell::new(Vec::new()));
        let stdout_buff = Rc::new(RefCell::new(Vec::new()));
        let stderr_buff = Rc::new(RefCell::new(Vec::new()));
        let stdin = ContainerStdin {};
        let stdout = ContainerStdout::new(Rc::clone(&iter), stdin_buff, stdout_buff, stderr_buff);
        let stderr = ContainerStderr {};
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
        use container::AttachResponseFrame::*;
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
            0 => Some(Ok(Stdin(frame))),
            1 => Some(Ok(Stdout(frame))),
            2 => Some(Ok(Stderr(frame))),
            n => {
                error!("unexpected kind of chunk: {}", n);
                None
            }
        }
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
