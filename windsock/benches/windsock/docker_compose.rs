use docker_compose_runner::{DockerCompose, Image};
use std::time::Duration;

pub fn docker_compose(file_path: &str) -> DockerCompose {
    DockerCompose::new(&IMAGE_WAITERS, |_| {}, file_path)
}

static IMAGE_WAITERS: [Image; 1] = [Image {
    name: "shotover/cassandra-test:4.0.6-r1",
    log_regex_to_wait_for: r"Startup complete",
    timeout: Duration::from_secs(120),
}];
