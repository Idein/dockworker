use dockworker::{container::ContainerFilters, Docker};
use tokio::task;

#[tokio::main]
async fn main() {
    task::spawn(async {
        let docker = Docker::connect_with_defaults().unwrap();
        let filter = ContainerFilters::new();
        let containers = docker
            .list_containers(None, None, None, filter)
            .await
            .unwrap();

        containers.iter().for_each(|c| {
            println!("{c:?}");
        });
    });    
}
