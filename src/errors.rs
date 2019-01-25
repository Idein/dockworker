//! Error-handling with the `error_chain` crate.

use base64;
use docker;
use http;
use hyper;
use response;
use serde_json;
use std::env;
use std::io;

error_chain! {
    foreign_links {
        EnvVar(env::VarError);
        Hyper(hyper::Error);
        Io(io::Error);
        Json(serde_json::error::Error);
        Docker(docker::DockerError);
        Base64(base64::DecodeError);
        DockerResponse(response::Error);
        Http(http::Error);
        HttpUriInvalidUri(http::uri::InvalidUri);
        HttpUriInvalidUriParts(http::uri::InvalidUriParts);
        HttpHeaderToStrError(http::header::ToStrError);
        MimeFromStrErr(mime::FromStrError);
        HyperTlsError(hyper_tls::Error) #[cfg(feature = "openssl")];
        OpenSSLErrorStack(openssl::error::ErrorStack) #[cfg(feature = "openssl")];
    }

    errors {
        ContainerInfo(id: String) {
            description("could not fetch information about container")
            display("could not fetch information about container '{}'", &id)
        }

        CouldNotConnect(host: String) {
            description("could not connect to Docker")
            display("could not connected to Docker at '{}'", &host)
        }

        NoCertPath {
            description("could not find DOCKER_CERT_PATH")
            display("could not find DOCKER_CERT_PATH")
        }

        ParseError(wanted: &'static str, input: String) {
            description("error parsing JSON from Docker")
            display("could not parse JSON for {} from Docker", wanted)
        }

        SslDisabled {
            description("Docker SSL support was disabled at compile time")
            display("Docker SSL support was disabled at compile time")
        }

        SslError(host: String) {
            description("could not connect to Docker using SSL")
            display("could not connect to Docker at '{}' using SSL", &host)
        }

        UnsupportedScheme(host: String) {
            description("unsupported Docker URL scheme")
            display("do not know how to connect to Docker at '{}'", &host)
        }

        Unknown(message: String) {
            description("unknown error")
            display("unknown error: {}", &message)
        }
    }
}
