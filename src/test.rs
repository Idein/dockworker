#![cfg(test)]

use crate::container::{Container, ContainerInfo, HealthState};
use crate::filesystem::FilesystemChange;
use crate::image::{Image, SummaryImage};
use crate::network::Network;
use crate::options::ImageLayer;
use crate::process::Top;
use crate::stats::Stats;
use crate::system::SystemInfo;
use crate::version::Version;

#[test]
fn get_containers() {
    let response = get_containers_response();
    assert!(serde_json::from_str::<Vec<Container>>(response).is_ok())
}

#[test]
fn get_networks() {
    let response = include_str!("fixtures/list_networks.json");
    assert!(serde_json::from_str::<Vec<Network>>(response).is_ok())
}

#[test]
fn get_stats_suspended() {
    let stats_oneshot = include_str!("fixtures/stats_suspend.json");
    let v = serde_json::from_str::<Stats>(stats_oneshot).unwrap();
    assert!(v.memory_stats.is_none());
}

#[tokio::test]
async fn get_stats_streaming() {
    let res = get_stats_response();
    let src = crate::docker::into_jsonlines::<Stats>(res.into_body()).unwrap();
    use futures::stream::StreamExt;
    let stats = src
        .collect::<Vec<_>>()
        .await
        .into_iter()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(stats.len(), 3);
    assert!(stats[0].memory_stats.is_some());
    assert!(stats[1].memory_stats.is_some());
    assert!(stats[2].memory_stats.is_some());
}

#[test]
fn get_system_info() {
    let response = get_system_info_response();
    assert!(serde_json::from_str::<SystemInfo>(response).is_ok())
}

#[test]
fn get_image_list() {
    let response = get_image_list_response();
    let images: Vec<SummaryImage> = serde_json::from_str(response).unwrap();
    assert_eq!(3, images.len());
}

#[test]
fn get_image() {
    let response = get_image_response();
    println!("response: {:?}", serde_json::from_str::<Image>(response));
}

#[test]
fn get_image_history() {
    let response = get_image_history_reponse();
    let images: Vec<ImageLayer> = serde_json::from_str(response).unwrap();
    assert_ne!(images[0].id, None);
    assert_eq!(2, images.len());
}

#[test]
fn get_container_info() {
    let response = get_container_info_response();
    serde_json::from_str::<ContainerInfo>(response).unwrap();

    let response = get_container_info_response_with_healthcheck();
    serde_json::from_str::<ContainerInfo>(response).unwrap();
}

#[test]
fn get_healthcheck_info() {
    let response = get_container_info_response_with_healthcheck();
    let container_info = serde_json::from_str::<ContainerInfo>(response).unwrap();
    assert!(container_info.State.Health.is_some());
    assert!(container_info.State.Health.unwrap().Status == HealthState::Healthy);
}

#[test]
fn get_processes() {
    let response = get_processes_response();
    assert!(serde_json::from_str::<Top>(response).is_ok())
}

#[test]
fn get_filesystem_changes() {
    let response = get_filesystem_changes_response();
    assert!(serde_json::from_str::<Vec<FilesystemChange>>(response).is_ok())
}

#[test]
fn get_version() {
    let response = get_version_response();
    assert!(serde_json::from_str::<Version>(response).is_ok())
}

fn get_containers_response() -> &'static str {
    include_str!("fixtures/containers_response.json")
}

fn get_system_info_response() -> &'static str {
    include_str!("fixtures/system_info.json")
}

// `docker inspect debian:wheely-2019- |  jq '.[]'
fn get_image_response() -> &'static str {
    include_str!("fixtures/image.json")
}

fn get_image_list_response() -> &'static str {
    include_str!("fixtures/image_list.json")
}

fn get_image_history_reponse() -> &'static str {
    // First has Id, second has Id missing.
    include_str!("fixtures/image_history.json")
}

fn get_container_info_response() -> &'static str {
    include_str!("fixtures/container_inspect.json")
}

fn get_container_info_response_with_healthcheck() -> &'static str {
    include_str!("fixtures/container_inspect_health.json")
}

fn get_processes_response() -> &'static str {
    include_str!("fixtures/processes.json")
}

fn get_filesystem_changes_response() -> &'static str {
    include_str!("fixtures/filesystem_changes.json")
}

fn get_version_response() -> &'static str {
    include_str!("fixtures/version.json")
}

fn get_stats_response() -> http::Response<hyper::Body> {
    let response = http::Response::builder()
        .status(http::StatusCode::OK)
        .header("Transfer-Encoding", "chunked")
        .header("Connection", "Close");
    let body = include_str!("fixtures/stats_stream.json").to_string();
    response.body(hyper::Body::from(body)).unwrap()
}

// docker run -d -p 5000:5000 registry
// docker push localhost:5000/hello-world:latest
static MANIFEST: &str = r###"{
   "schemaVersion": 2,
   "mediaType": "application/vnd.docker.distribution.manifest.v2+json",
   "config": {
      "mediaType": "application/vnd.docker.container.image.v1+json",
      "size": 1470,
      "digest": "sha256:9c7a54a9a43cca047013b82af109fe963fde787f63f9e016fdc3384500c2823d"
   },
   "layers": [
      {
         "mediaType": "application/vnd.docker.image.rootfs.diff.tar.gzip",
         "size": 2457,
         "digest": "sha256:719385e32844401d57ecfd3eacab360bf551a1491c05b85806ed8f1b08d792f6"
      }
   ]
}"###;
static CONFIG: &str = r###"eyJhcmNoaXRlY3R1cmUiOiJhbWQ2NCIsImNvbmZpZyI6eyJIb3N0bmFtZSI6IiIsIkRvbWFpbm5h
bWUiOiIiLCJVc2VyIjoiIiwiQXR0YWNoU3RkaW4iOmZhbHNlLCJBdHRhY2hTdGRvdXQiOmZhbHNl
LCJBdHRhY2hTdGRlcnIiOmZhbHNlLCJUdHkiOmZhbHNlLCJPcGVuU3RkaW4iOmZhbHNlLCJTdGRp
bk9uY2UiOmZhbHNlLCJFbnYiOlsiUEFUSD0vdXNyL2xvY2FsL3NiaW46L3Vzci9sb2NhbC9iaW46
L3Vzci9zYmluOi91c3IvYmluOi9zYmluOi9iaW4iXSwiQ21kIjpbIi9oZWxsbyJdLCJJbWFnZSI6
InNoYTI1Njo2MmExNTYxOTAzN2YzYzRmYjRlNmJhOWJkMjI0Y2JhMzU0MGUzOTNhNTVkYzUyZjZi
ZWJlMjEyY2E3YjVlMWE3IiwiVm9sdW1lcyI6bnVsbCwiV29ya2luZ0RpciI6IiIsIkVudHJ5cG9p
bnQiOm51bGwsIk9uQnVpbGQiOm51bGwsIkxhYmVscyI6bnVsbH0sImNvbnRhaW5lciI6IjM0N2Nh
Njg4NzJlZTkyNGM0ZjkzOTRiMTk1ZGNhZGFmNTkxZDM4N2E0NWQ2MjQyMjUyNTFlZmM2Y2I3YTM0
OGUiLCJjb250YWluZXJfY29uZmlnIjp7Ikhvc3RuYW1lIjoiMzQ3Y2E2ODg3MmVlIiwiRG9tYWlu
bmFtZSI6IiIsIlVzZXIiOiIiLCJBdHRhY2hTdGRpbiI6ZmFsc2UsIkF0dGFjaFN0ZG91dCI6ZmFs
c2UsIkF0dGFjaFN0ZGVyciI6ZmFsc2UsIlR0eSI6ZmFsc2UsIk9wZW5TdGRpbiI6ZmFsc2UsIlN0
ZGluT25jZSI6ZmFsc2UsIkVudiI6WyJQQVRIPS91c3IvbG9jYWwvc2JpbjovdXNyL2xvY2FsL2Jp
bjovdXNyL3NiaW46L3Vzci9iaW46L3NiaW46L2JpbiJdLCJDbWQiOlsiL2Jpbi9zaCIsIi1jIiwi
Iyhub3ApICIsIkNNRCBbXCIvaGVsbG9cIl0iXSwiSW1hZ2UiOiJzaGEyNTY6NjJhMTU2MTkwMzdm
M2M0ZmI0ZTZiYTliZDIyNGNiYTM1NDBlMzkzYTU1ZGM1MmY2YmViZTIxMmNhN2I1ZTFhNyIsIlZv
bHVtZXMiOm51bGwsIldvcmtpbmdEaXIiOiIiLCJFbnRyeXBvaW50IjpudWxsLCJPbkJ1aWxkIjpu
dWxsLCJMYWJlbHMiOnt9fSwiY3JlYXRlZCI6IjIwMjMtMDUtMDRUMTc6Mzc6MDMuODcyOTU4NzEy
WiIsImRvY2tlcl92ZXJzaW9uIjoiMjAuMTAuMjMiLCJoaXN0b3J5IjpbeyJjcmVhdGVkIjoiMjAy
My0wNS0wNFQxNzozNzowMy44MDE4NDA4MjNaIiwiY3JlYXRlZF9ieSI6Ii9iaW4vc2ggLWMgIyhu
b3ApIENPUFkgZmlsZToyMDFmOGYxODQ5ZTg5ZDUzYmU5ZjZhYTc2OTM3ZjVlMjA5ZDc0NWFiZmQx
NWE4NTUyZmNmMmJhNDVhYjI2N2Y5IGluIC8gIn0seyJjcmVhdGVkIjoiMjAyMy0wNS0wNFQxNzoz
NzowMy44NzI5NTg3MTJaIiwiY3JlYXRlZF9ieSI6Ii9iaW4vc2ggLWMgIyhub3ApICBDTUQgW1wi
L2hlbGxvXCJdIiwiZW1wdHlfbGF5ZXIiOnRydWV9XSwib3MiOiJsaW51eCIsInJvb3RmcyI6eyJ0
eXBlIjoibGF5ZXJzIiwiZGlmZl9pZHMiOlsic2hhMjU2OjAxYmI0ZmNlM2ViMWI1NmIwNWFkZjk5
NTA0ZGFmZDMxOTA3YTVhYWRhYzczNmUzNmIyNzU5NWM4YjkyZjA3ZjEiXX19"###;
static LAYER: &str = r###"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
lWQAOgAA
"###;
fn v2_version(req: &hyper::Request<hyper::Body>) -> Option<hyper::Response<hyper::Body>> {
    match (req.method(), req.uri().path()) {
        (&hyper::Method::GET, "/v2/") => {
            // curl -X GET -H "Accept: application/vnd.docker.distribution.manifest.v2+json" -s -D - http://localhost:5000/v2/
            let res = hyper::Response::builder()
                .status(hyper::StatusCode::OK)
                .header(
                    "Content-Type",
                    "Content-Type: application/json; charset=utf-8",
                )
                .header("Docker-Distribution-Api-Version", "registry/2.0")
                .header("X-Content-Type-Options", "nosniff")
                .header("Content-Length", "2")
                .body(hyper::Body::from("{}"))
                .unwrap();
            Some(res)
        }
        _ => None,
    }
}
fn v2_manifest_latest(req: &hyper::Request<hyper::Body>) -> Option<hyper::Response<hyper::Body>> {
    match (req.method(), req.uri().path()) {
        (&hyper::Method::HEAD, "/v2/hello-world/manifests/latest") => {
            // curl -X HEAD -H "Accept: application/vnd.docker.distribution.manifest.v2+json" -s -D - http://localhost:5000/v2/hello-world/manifests/latest
            let res = hyper::Response::builder()
                .status(hyper::StatusCode::OK)
                .header(
                    "Content-Type",
                    "application/vnd.docker.distribution.manifest.v2+json",
                )
                .header(
                    "Docker-Content-Digest",
                    "sha256:7e9b6e7ba2842c91cf49f3e214d04a7a496f8214356f41d81a6e6dcad11f11e3",
                )
                .header("Docker-Distribution-Api-Version", "registry/2.0")
                .header("X-Content-Type-Options", "nosniff")
                .header("Content-Length", "525")
                .body(hyper::Body::empty())
                .unwrap();
            Some(res)
        }
        _ => None,
    }
}
fn v2_manifest_sha256(
    req: &hyper::Request<hyper::Body>,
    conn_shutdown: bool,
) -> Option<hyper::Response<hyper::Body>> {
    match (req.method(), req.uri().path()) {
            (&hyper::Method::GET, "/v2/hello-world/manifests/sha256:7e9b6e7ba2842c91cf49f3e214d04a7a496f8214356f41d81a6e6dcad11f11e3") => {
                let body = MANIFEST.as_bytes();
                assert_eq!(body.len(), 525);
                let body = async_stream::stream!{
                    for (_i, bytes) in body.chunks(16).enumerate() {
                        if conn_shutdown {
                            // simulate a connection shutdown
                            return;
                        }
                        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                        yield Ok(bytes.to_vec()) as Result<_, std::io::Error>;
                    }
                };
                // curl -X GET -H "Accept: application/vnd.docker.distribution.manifest.v2+json" -s -D - http://localhost:5000/v2/hello-world/manifests/sha256:7e9b6e7ba2842c91cf49f3e214d04a7a496f8214356f41d81a6e6dcad11f11e3
                let res = hyper::Response::builder()
                    .status(hyper::StatusCode::OK)
                    .header("Content-Type", "application/vnd.docker.distribution.manifest.v2+json")
                    .header("Docker-Content-Digest", "sha256:7e9b6e7ba2842c91cf49f3e214d04a7a496f8214356f41d81a6e6dcad11f11e3")
                    .header("Docker-Distribution-Api-Version", "registry/2.0")
                    .header("X-Content-Type-Options", "nosniff")
                    .header("Content-Length", "525")
                    .body(hyper::Body::wrap_stream(body))
                    .unwrap();
                Some(res)
            },
            _ => { None }
        }
}
fn v2_blob_config(
    req: &hyper::Request<hyper::Body>,
    conn_shutdown: bool,
) -> Option<hyper::Response<hyper::Body>> {
    match (req.method(), req.uri().path()) {
            (&hyper::Method::GET, "/v2/hello-world/blobs/sha256:9c7a54a9a43cca047013b82af109fe963fde787f63f9e016fdc3384500c2823d") => {
                use base64::Engine as _;
                let body = CONFIG.lines().collect::<String>();
                let body = base64::engine::general_purpose::STANDARD_NO_PAD.decode(body).unwrap();
                assert_eq!(body.len(), 1470);
                let body = async_stream::stream!{
                    for (_i, bytes) in body.chunks(16).enumerate() {
                        if conn_shutdown {
                            // simulate a connection shutdown
                            return;
                        }
                        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                        yield Ok(bytes.to_vec()) as Result<_, std::io::Error>;
                    }
                };
                // curl -X GET -H "Accept: application/vnd.docker.distribution.manifest.v2+json" -s -D - http://localhost:5000/v2/hello-world/blobs/sha256:9c7a54a9a43cca047013b82af109fe963fde787f63f9e016fdc3384500c2823d
                let res = hyper::Response::builder()
                    .status(hyper::StatusCode::OK)
                    .header("Accept-Ranges", "bytes")
                    .header("Content-Type", "application/octet-stream")
                    .header("Docker-Content-Digest", "sha256:9c7a54a9a43cca047013b82af109fe963fde787f63f9e016fdc3384500c2823d")
                    .header("Docker-Distribution-Api-Version", "registry/2.0")
                    .header("X-Content-Type-Options", "nosniff")
                    .header("Content-Length", "1470")
                    .body(hyper::Body::wrap_stream(body))
                    .unwrap();
                Some(res)
            },
            _ => { None }
        }
}
fn v2_blob_layer(
    req: &hyper::Request<hyper::Body>,
    conn_shutdown_count: Option<usize>,
) -> Option<hyper::Response<hyper::Body>> {
    match (req.method(), req.uri().path()) {
            (&hyper::Method::GET, "/v2/hello-world/blobs/sha256:719385e32844401d57ecfd3eacab360bf551a1491c05b85806ed8f1b08d792f6") => {
                dbg!(&req);
                use base64::Engine as _;
                let body = LAYER.lines().collect::<String>();
                let body = base64::engine::general_purpose::STANDARD_NO_PAD.decode(body).unwrap();
                assert_eq!(body.len(), 2457);
                let body = async_stream::stream!{
                    for (i, bytes) in body.chunks(16).enumerate() {
                        if let Some(count) = conn_shutdown_count {
                            if i > count {
                                // simulate a connection shutdown
                                return;
                            }
                        }
                        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                        yield Ok(bytes.to_vec()) as Result<_, std::io::Error>;
                    }
                };
                // curl -X GET -H "Accept: application/vnd.docker.distribution.manifest.v2+json" -s -D - http://localhost:5000/v2/hello-world/blobs/sha256:719385e32844401d57ecfd3eacab360bf551a1491c05b85806ed8f1b08d792f6
                let res = hyper::Response::builder()
                    .status(hyper::StatusCode::OK)
                    .header("Content-Type", "application/octet-stream")
                    .header("Docker-Content-Digest", "sha256:719385e32844401d57ecfd3eacab360bf551a1491c05b85806ed8f1b08d792f6")
                    .header("Docker-Distribution-Api-Version", "registry/2.0")
                    .header("X-Content-Type-Options", "nosniff")
                    .header("Content-Length", "2457")
                    .body(hyper::Body::wrap_stream(body))
                    .unwrap();
                Some(res)
            },
            _ => { None }
        }
}
#[tokio::test]
#[serial_test::serial]
async fn pull_succ() {
    let dw = crate::Docker::connect_with_defaults().unwrap();
    let pass = crate::credentials::UserPassword::new(
        "".to_string(),
        "".to_string(),
        "".to_string(),
        "".to_string(),
    );
    let cred = crate::credentials::Credential::with_password(pass);
    dw.set_credential(cred);
    let image_id = "localhost:3000/hello-world:latest";
    dw.remove_image(image_id, Some(true), Some(true)).await.ok();
    let (server_ready_tx, server_ready_rx) = tokio::sync::oneshot::channel::<()>();
    let (shutdown_server_tx, shutdown_server_rx) = tokio::sync::oneshot::channel::<()>();
    let fut1 = async {
        let app = hyper::service::make_service_fn(|_conn| async move {
            let svc =
                hyper::service::service_fn(move |req: hyper::Request<hyper::Body>| async move {
                    v2_version(&req)
                        .or_else(|| v2_manifest_latest(&req))
                        .or_else(|| v2_manifest_sha256(&req, false))
                        .or_else(|| v2_blob_config(&req, false))
                        .or_else(|| v2_blob_layer(&req, None))
                        .ok_or_else(|| format!("abort: {req:?}"))
                });
            Ok(svc) as Result<_, std::convert::Infallible>
        });
        let fut = hyper::Server::bind(&std::net::SocketAddr::from(([0, 0, 0, 0], 3000))).serve(app);
        let fut = futures::future::join(
            async {
                fut.await.unwrap();
            },
            async {
                server_ready_tx.send(()).unwrap();
            },
        );
        tokio::select! {
            _ = fut => { unreachable!() },
            _ = shutdown_server_rx => {},
        }
    };
    let fut2 = async {
        server_ready_rx.await.unwrap();
        // manifest の取得までは POST /images/create の response header を返す前に行われる
        // manifest は containerd が /var/lib/containerd 以下にキャッシュしており、 docker rmi
        // では削除できない
        // その後 response body の json stream で blob の download が行われる
        let res = dw.create_image(image_id, "").await.unwrap();
        use futures::StreamExt;
        res.enumerate()
            .for_each(|(i, msg)| async move {
                println!("{i}:{msg:?}");
            })
            .await;
        shutdown_server_tx.send(()).unwrap();
    };
    futures::future::join(fut1, fut2).await;
    dw.remove_image(image_id, Some(true), Some(true)).await.ok();
}
#[rstest::rstest]
#[case(
    "cannot_lookup",
    "Get https://unexist.actcast.io/v2/: dial tcp: lookup unexist.actcast.io: no such host"
)]
#[case(
    "cannot_connect",
    "Get http://localhost:3001/v2/: dial tcp 127.0.0.1:3001: connect: connection refused"
)]
#[case("cannot_get_version", "Get http://localhost:3000/v2/: EOF")]
#[case(
    "cannot_get_manifest",
    "Head http://localhost:3000/v2/hello-world/manifests/latest: EOF"
)]
#[case("cannot_get_manifest_sha256", "Get http://localhost:3000/v2/hello-world/manifests/sha256:7e9b6e7ba2842c91cf49f3e214d04a7a496f8214356f41d81a6e6dcad11f11e3: EOF")]
// this test sometimes failed if manifest file is cached
#[case("cannot_download_manifest_sha256", "Get http://localhost:3000/v2/hello-world/manifests/sha256:7e9b6e7ba2842c91cf49f3e214d04a7a496f8214356f41d81a6e6dcad11f11e3: EOF")]
#[serial_test::serial]
#[tokio::test]
async fn pull_failed_before_stream_response(
    #[case] case: &'static str,
    #[case] message: &'static str,
) {
    let dw = crate::Docker::connect_with_defaults().unwrap();
    let pass = crate::credentials::UserPassword::new(
        "".to_string(),
        "".to_string(),
        "".to_string(),
        "".to_string(),
    );
    let cred = crate::credentials::Credential::with_password(pass);
    dw.set_credential(cred);
    let image_id = match case {
        "cannot_lookup" => "unexist.actcast.io/hello-world:latest",
        "cannot_connect" => "localhost:3001/hello-world:latest",
        _ => "localhost:3000/hello-world:latest",
    };
    dw.remove_image(image_id, Some(true), Some(true)).await.ok();
    let (server_ready_tx, server_ready_rx) = tokio::sync::oneshot::channel::<()>();
    let (shutdown_server_tx, shutdown_server_rx) = tokio::sync::oneshot::channel::<()>();
    let fut1 = async {
        let app = hyper::service::make_service_fn(|_conn| async move {
            let svc =
                hyper::service::service_fn(move |req: hyper::Request<hyper::Body>| async move {
                    match case {
                        "cannot_get_version" => Err("abort"),
                        "cannot_get_manifest" => v2_version(&req).ok_or("abort"),
                        "cannot_get_manifest_sha256" => v2_version(&req)
                            .or_else(|| v2_manifest_latest(&req))
                            .ok_or("abort"),
                        "cannot_download_manifest_sha256" => v2_version(&req)
                            .or_else(|| v2_manifest_latest(&req))
                            .or_else(|| v2_manifest_sha256(&req, true))
                            .ok_or("abort"),
                        _ => unreachable!(),
                    }
                });
            Ok(svc) as Result<_, std::convert::Infallible>
        });
        let fut = hyper::Server::bind(&std::net::SocketAddr::from(([0, 0, 0, 0], 3000))).serve(app);
        let fut = futures::future::join(
            async {
                fut.await.unwrap();
            },
            async {
                server_ready_tx.send(()).unwrap();
            },
        );
        tokio::select! {
            _ = fut => { unreachable!() },
            _ = shutdown_server_rx => {},
        }
    };
    let fut2 = async {
        server_ready_rx.await.unwrap();
        // manifest の取得までは POST /images/create の response header を返す前に行われる
        // manifest は containerd が /var/lib/containerd 以下にキャッシュしており、 docker rmi
        // では削除できない
        // その後 response body の json stream で blob の download が行われる
        let res = dw.create_image(image_id, "").await;
        if let Err(crate::errors::Error::Docker(err)) = res {
            dbg!(&err);
            assert_eq!(err.message, message);
        } else {
            panic!("assertion failed");
        }
        shutdown_server_tx.send(()).unwrap();
    };
    futures::future::join(fut1, fut2).await;
    dw.remove_image(image_id, Some(true), Some(true)).await.ok();
}

#[rstest::rstest]
#[case(true, 999, "error pulling image configuration: Get http://localhost:3000/v2/hello-world/blobs/sha256:9c7a54a9a43cca047013b82af109fe963fde787f63f9e016fdc3384500c2823d: EOF", "error pulling image configuration: Get http://localhost:3000/v2/hello-world/blobs/sha256:9c7a54a9a43cca047013b82af109fe963fde787f63f9e016fdc3384500c2823d: EOF")]
#[case(false, 0, "unexpected EOF", "unexpected EOF")]
#[case(
    false,
    1,
    "expected HTTP 206 from byte range request",
    "expected HTTP 206 from byte range request"
)]
#[serial_test::serial]
#[tokio::test]
async fn pull_failed_on_response_stream(
    #[case] v2_blob_config_flag: bool,
    #[case] v2_blob_layer_count: usize,
    #[case] error: &str,
    #[case] message: &str,
) {
    let dw = crate::Docker::connect_with_defaults().unwrap();
    let pass = crate::credentials::UserPassword::new(
        "".to_string(),
        "".to_string(),
        "".to_string(),
        "".to_string(),
    );
    let cred = crate::credentials::Credential::with_password(pass);
    dw.set_credential(cred);
    let image_id = "localhost:3000/hello-world:latest";
    dw.remove_image(image_id, Some(true), Some(true)).await.ok();
    let (server_ready_tx, server_ready_rx) = tokio::sync::oneshot::channel::<()>();
    let (shutdown_server_tx, shutdown_server_rx) = tokio::sync::oneshot::channel::<()>();
    let fut1 = async {
        let app = hyper::service::make_service_fn(|_conn| async move {
            let svc =
                hyper::service::service_fn(move |req: hyper::Request<hyper::Body>| async move {
                    v2_version(&req)
                        .or_else(|| v2_manifest_latest(&req))
                        .or_else(|| v2_manifest_sha256(&req, false))
                        .or_else(|| v2_blob_config(&req, v2_blob_config_flag))
                        .or_else(|| v2_blob_layer(&req, Some(v2_blob_layer_count)))
                        .ok_or("abort")
                });
            Ok(svc) as Result<_, std::convert::Infallible>
        });
        let fut = hyper::Server::bind(&std::net::SocketAddr::from(([0, 0, 0, 0], 3000))).serve(app);
        let fut = futures::future::join(
            async {
                fut.await.unwrap();
            },
            async {
                server_ready_tx.send(()).unwrap();
            },
        );
        tokio::select! {
            _ = fut => { unreachable!() },
            _ = shutdown_server_rx => {},
        }
    };
    let fut2 = async {
        server_ready_rx.await.unwrap();
        let res = dw.create_image(image_id, "").await.unwrap();
        use futures::StreamExt;
        let results = res
            .enumerate()
            .map(move |(i, msg)| {
                println!("{i}:{msg:?}");
                msg
            })
            .collect::<Vec<_>>()
            .await;
        let ret = results.into_iter().any(|msg| {
            use crate::response::Error;
            use crate::response::ErrorDetail;
            use crate::response::Response;
            if let Ok(msg) = msg {
                msg == Response::Error(Error {
                    error: error.to_string(),
                    errorDetail: ErrorDetail {
                        message: message.to_string(),
                    },
                })
            } else {
                false
            }
        });
        assert!(ret);
        shutdown_server_tx.send(()).unwrap();
    };
    futures::future::join(fut1, fut2).await;
    dw.remove_image(image_id, Some(true), Some(true)).await.ok();
}
