///! Response from Dockerd
///!
use serde::{Deserialize, Serialize};
use serde_json::value as json;
use std::error::Error as StdError;
use std::fmt;

#[derive(Debug, PartialEq, Eq, PartialOrd, Clone, Serialize, Deserialize)]
pub struct ProgressDetail {
    pub current: u64,
    pub total: u64,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Clone, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct Progress {
    /// image tag or hash of image layer or ...
    pub id: String,
    /// progress bar
    pub progress: Option<String>,
    /// progress detail
    #[serde(deserialize_with = "progress_detail_opt::deserialize")]
    pub progressDetail: Option<ProgressDetail>,
    /// message or auxiliary info
    pub status: String,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Clone, Serialize, Deserialize)]
pub struct Stream {
    pub stream: String,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Clone, Serialize, Deserialize)]
pub struct Status {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub status: String,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Clone, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct LogID {
    pub ID: String,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Clone, Serialize, Deserialize)]
pub struct Aux {
    pub aux: LogID,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Clone, Serialize, Deserialize)]
pub struct LogResponse {
    pub response: String,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Clone, Serialize, Deserialize)]
pub struct ErrorDetail {
    pub message: String,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Clone, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct Error {
    pub error: String,
    pub errorDetail: ErrorDetail,
}

/// Response of /images/create or other API
///
/// ## NOTE
/// Structure of this type is not documented officialy.
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Response {
    Progress(Progress),
    Status(Status),
    Stream(Stream),
    Aux(Aux),
    Response(LogResponse),
    Error(Error),
    /// unknown response
    Unknown(json::Value),
}

impl fmt::Display for Error {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(fmt, "{}: {}", self.error, self.errorDetail.message)
    }
}

impl StdError for Error {
    fn description(&self) -> &str {
        &self.error
    }

    fn cause(&self) -> Option<&(dyn std::error::Error + 'static)> {
        None
    }
}

impl Response {
    pub fn as_error(&self) -> Option<&Error> {
        use self::Response::*;
        if let Error(err) = self {
            Some(err)
        } else {
            None
        }
    }
}

mod progress_detail_opt {
    use super::*;
    use serde::de::{self, Deserializer, MapAccess, Visitor};

    pub fn deserialize<'de, D>(de: D) -> Result<Option<ProgressDetail>, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct OptVisitor;

        impl<'de> Visitor<'de> for OptVisitor {
            type Value = Option<ProgressDetail>;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("Option<ProgressDetail>")
            }

            fn visit_map<V>(self, mut map: V) -> Result<Self::Value, V::Error>
            where
                V: MapAccess<'de>,
            {
                let mut current = None;
                let mut total = None;

                match map.next_key()? {
                    Some(mut key) => loop {
                        match key {
                            "current" => {
                                if current.is_some() {
                                    return Err(de::Error::duplicate_field("current"));
                                }
                                current = Some(map.next_value()?);
                            }
                            "total" => {
                                if total.is_some() {
                                    return Err(de::Error::duplicate_field("total"));
                                }
                                total = Some(map.next_value()?);
                            }
                            _ => return Err(de::Error::unknown_field(key, FIELDS)),
                        };
                        if let Some(key_) = map.next_key()? {
                            key = key_;
                        } else {
                            break;
                        }
                    },
                    None => return Ok(None), // {}
                }

                let current = current.ok_or_else(|| de::Error::missing_field("current"))?;
                let total = total.ok_or_else(|| de::Error::missing_field("total"))?;

                Ok(Some(ProgressDetail { current, total }))
            }
        }

        const FIELDS: &[&str] = &["current", "total"];
        de.deserialize_map(OptVisitor)
    }
}

#[cfg(test)]
mod tests {
    use self::Response as R;
    use super::*;
    use serde_json;

    #[test]
    #[rustfmt::skip]
    fn progress() {
        let s = r#"{
            "status": "Downloading",
            "progressDetail":{
                "current":1596117,
                "total":86451485
            },
            "progress":"[\u003e                                                  ]  1.596MB/86.45MB","id":"66aa7ce9b58b"
        }"#;
        assert_eq!(
            R::Progress(Progress {
                id: "66aa7ce9b58b".to_owned(),
                progress:
                    "[\u{003e}                                                  ]  1.596MB/86.45MB"
                        .to_owned()
                        .into(),
                status: "Downloading".to_owned(),
                progressDetail: Some(ProgressDetail {
                    current: 1596117,
                    total: 86451485,
                }),
            }),
            serde_json::from_str(s).unwrap()
        )
    }

    #[test]
    fn progress_empty() {
        let s = r#"{"status":"Already exists","progressDetail":{},"id":"18b8eb7e7f01"}"#;
        assert_eq!(
            Progress {
                id: "18b8eb7e7f01".to_owned(),
                progress: None,
                progressDetail: None,
                status: "Already exists".to_owned(),
            },
            serde_json::from_str(s).unwrap()
        );
    }

    #[test]
    fn status() {
        let s = r#"{"status":"Pulling from eldesh/smlnj","id":"110.78"}"#;
        assert_eq!(
            R::Status(Status {
                id: Some("110.78".to_owned()),
                status: "Pulling from eldesh/smlnj".to_owned(),
            }),
            serde_json::from_str(s).unwrap()
        )
    }

    #[test]
    #[rustfmt::skip]
    fn error() {
        let s = r#"{
            "errorDetail":{
                "message":"failed to register layer: Error processing tar file(exit status 1): write /foo/bar: no space left on device"
            },
            "error":"failed to register layer: Error processing tar file(exit status 1): write /foo/bar: no space left on device"
        }"#;
        assert_eq!(
            R::Error(Error {
                error: "failed to register layer: Error processing tar file(exit status 1): write /foo/bar: no space left on device".to_owned(),
                errorDetail: ErrorDetail {
                    message: "failed to register layer: Error processing tar file(exit status 1): write /foo/bar: no space left on device".to_owned(),
                },
            }),
            serde_json::from_str(s).unwrap()
        )
    }
}
