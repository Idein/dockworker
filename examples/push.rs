use dockworker::{
    credentials::{Credential, UserPassword},
    Docker,
};

#[tokio::main]
async fn main() {
    let docker = Docker::connect_with_defaults().unwrap();

    let (name, tag) = ("alpine", "latest");
    docker.create_image(name, tag).await.unwrap();

    let serveraddress = "localhost:5000";
    docker.set_credential(Credential::with_password(UserPassword::new(
        "someusername".to_owned(),
        "somepassword".to_owned(),
        "someusername@example.com".to_owned(),
        serveraddress.to_owned(),
    )));

    println!("pulled: {name}:{tag}");
    docker
        .push_image(&format!("{serveraddress}/{name}"), tag)
        .await
        .unwrap();
    println!("pushed: {name}:{tag}");
}
