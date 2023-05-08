use async_trait::async_trait;
use docker_compose_runner::{DockerCompose, Image};
use scylla::transport::Compression;
use scylla::SessionBuilder;
use std::path::Path;
use tokio::sync::mpsc::UnboundedSender;
use windsock::{Bench, Report, Tags, Windsock};

fn main() {
    set_working_dir();
    Windsock::new(
        vec![
            Box::new(CassandraBench::new(Some(Compression::Lz4))),
            Box::new(CassandraBench::new(None)),
        ],
        &["release"],
    )
    .run();
}

struct CassandraBench {
    compression: Option<Compression>,
}

impl CassandraBench {
    fn new(compression: Option<Compression>) -> Self {
        CassandraBench { compression }
    }
}

#[async_trait]
impl Bench for CassandraBench {
    fn tags(&self) -> Tags {
        [
            ("name".to_owned(), "cassandra".to_owned()),
            ("topology".to_owned(), "3_nodes".to_owned()),
            ("OPS".to_owned(), "1000".to_owned()),
            ("message_type".to_owned(), "write1000bytes".to_owned()),
            (
                "compression".to_owned(),
                match &self.compression {
                    Some(Compression::Lz4) => "LZ4".to_owned(),
                    Some(Compression::Snappy) => "Snappy".to_owned(),
                    None => "None".to_owned(),
                },
            ),
        ]
        .into_iter()
        .collect()
    }

    async fn run(&self, flamegraph: bool, local: bool, reporter: UnboundedSender<Report>) {
        let _docker_compose = if local {
            docker_compose("examples/cassandra-docker-compose.yaml")
        } else {
            todo!("create instances on real infrastructure that reflects production use, that might mean spinning up instances on AWS or deploying and using physical infrastructure")
        };
        if flamegraph {
            todo!("run flamegraph");
        }

        let session = SessionBuilder::new()
            .known_nodes(&["172.16.1.2:9042"])
            .user("cassandra", "cassandra")
            .compression(self.compression)
            .build()
            .await
            .unwrap();

        // TODO: run benchmark, sending reports as it goes
        session
            .query("SELECT * FROM system.peers", ())
            .await
            .unwrap();
        reporter.send(Report::ThingHappen).unwrap();
    }
}

fn docker_compose(file_path: &str) -> DockerCompose {
    DockerCompose::new(get_image_waiters(), |_| {}, file_path)
}

fn get_image_waiters() -> &'static [Image] {
    &[Image {
        name: "bitnami/cassandra:4.0.6",
        log_regex_to_wait_for: r"Startup complete",
    }]
}

fn set_working_dir() {
    // tests and benches will set the directory to the directory of the crate, we are acting as a benchmark so we do the same
    std::env::set_current_dir(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join(env!("CARGO_PKG_NAME")),
    )
    .unwrap();
}