use dockworker::Docker;

#[tokio::main]
async fn main() {
    let docker = Docker::connect_with_defaults().unwrap();
    let token = docker
        .auth(
            "someusername",
            "somepassword",
            "someusername@example.com",
            "localhost:5000",
        )
        .await
        .unwrap();
    println!("token: {token:?}");
}
