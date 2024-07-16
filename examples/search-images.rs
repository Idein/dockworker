use std::env;

use dockworker::{image::ImageFilters, Docker};

#[tokio::main]
async fn main() {
    let docker = Docker::connect_with_defaults().unwrap();
    let mut args = env::args();
    let _ = args.next();
    let term = args.next().unwrap();
    let mut limit = None;

    let filter = {
        let mut filter = ImageFilters::new();
        for arg in args {
            if let Some(val) = arg.strip_prefix("--limit=") {
                limit = Some(val.parse::<u64>().unwrap());
            } else if let Some(val) = arg.strip_prefix("is-official=") {
                filter.is_official(val.parse::<bool>().unwrap());
            } else if let Some(val) = arg.strip_prefix("is-automated=") {
                filter.is_automated(val.parse::<bool>().unwrap());
            }
        }
        filter
    };

    for image in docker.search_images(&term, limit, filter).await.unwrap() {
        let official = if image.is_official { "[OFFICIAL]" } else { "" };
        let automate = if image.is_automated {
            "[AUTOMATED]"
        } else {
            ""
        };
        println!(
            "{} [{}] [{}⭐] {} {}",
            image.name, image.description, image.star_count, official, automate
        );
    }
}
