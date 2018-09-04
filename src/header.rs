///! HTTP header used in docker api
///!

use std::fmt;
use hyper::header::{Header, HeaderFormat};
use hyper::error::Result;
use hyper::Error;
use serde_json;
use base64::{self, MIME};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct XRegistryAuth {
    username: String,
    password: String,
    email: String,
    serveraddress: String,
}

impl XRegistryAuth {
    pub fn new(username: String, password: String, email: String, serveraddress: String) -> Self {
        Self {
            username,
            password,
            email,
            serveraddress,
        }
    }
}

impl Header for XRegistryAuth {
    fn header_name() -> &'static str {
        "X-Registry-Auth"
    }

    fn parse_header(raw: &[Vec<u8>]) -> Result<Self> {
        if raw.len() != 1 {
            return Err(Error::Header);
        }

        base64::decode_config(&raw[0], MIME)
            .map_err(|_| Error::Header)
            .and_then(|vec| {
                serde_json::from_str(&String::from_utf8_lossy(&vec)).map_err(|_| Error::Header)
            })
    }
}

impl HeaderFormat for XRegistryAuth {
    fn fmt_header(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let json = serde_json::to_string(self).map_err(|_| fmt::Error)?;
        let b64 = base64::encode_config(json.as_bytes(), MIME);
        write!(f, "{}", b64)
    }
}
