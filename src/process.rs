use serde::{Deserialize, Serialize};
use std::fmt::Error;
use std::fmt::{Display, Formatter};

#[derive(Debug, Default)]
pub struct Process {
    pub user: String,
    pub pid: String,
    pub cpu: Option<String>,
    pub memory: Option<String>,
    pub vsz: Option<String>,
    pub rss: Option<String>,
    pub tty: Option<String>,
    pub stat: Option<String>,
    pub start: Option<String>,
    pub time: Option<String>,
    pub command: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct Top {
    pub Titles: Vec<String>,
    pub Processes: Vec<Vec<String>>,
}

impl Display for Process {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        let mut s = String::new();

        s.push_str(&self.user.clone());

        s.push(',');
        s.push_str(&self.pid.clone());

        if let Some(v) = self.cpu.clone() {
            s.push(',');
            s.push_str(&v);
        }

        if let Some(v) = self.memory.clone() {
            s.push(',');
            s.push_str(&v);
        }

        if let Some(v) = self.vsz.clone() {
            s.push(',');
            s.push_str(&v);
        }

        if let Some(v) = self.rss.clone() {
            s.push(',');
            s.push_str(&v);
        }

        if let Some(v) = self.tty.clone() {
            s.push(',');
            s.push_str(&v);
        }

        if let Some(v) = self.stat.clone() {
            s.push(',');
            s.push_str(&v);
        }

        if let Some(v) = self.start.clone() {
            s.push(',');
            s.push_str(&v);
        }

        if let Some(v) = self.time.clone() {
            s.push(',');
            s.push_str(&v);
        }

        s.push(',');
        s.push_str(&self.command.clone());

        write!(f, "{s}")
    }
}
