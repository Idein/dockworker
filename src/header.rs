///! HTTP header used in docker api
///!

use std::fmt;
use hyper::header::{Header, HeaderFormat};
use hyper::error::Result;
use hyper::Error;
use base64::{self, STANDARD};

/// The http header represent `X-Registry-Auth`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct XRegistryAuth {
    /// Header value string.
    /// This body is sent/recv with enc/dec-ed base64 implicitly
    body: String,
}

impl XRegistryAuth {
    pub fn new(body: String) -> Self {
        Self { body }
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

        base64::decode_config(&raw[0], STANDARD)
            .map_err(|_| Error::Header)
            .map(|vec| Self::new(String::from_utf8_lossy(&vec).to_string()))
    }
}

impl HeaderFormat for XRegistryAuth {
    fn fmt_header(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let b64 = base64::encode_config(self.body.as_bytes(), STANDARD);
        debug!("{}: {}", Self::header_name(), b64);
        write!(f, "{}", b64)
    }
}
