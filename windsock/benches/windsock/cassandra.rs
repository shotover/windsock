use crate::docker_compose::docker_compose;
use anyhow::Result;
use async_trait::async_trait;
use scylla::SessionBuilder;
use scylla::{transport::Compression, Session};
use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::mpsc::UnboundedSender;
use windsock::{Bench, BenchParameters, BenchTask, Profiling, Report};

#[derive(Clone, Copy)]
pub enum Topology {
    Single,
    Cluster3,
}

impl Topology {
    pub fn to_tag(self) -> (String, String) {
        (
            "topology".to_owned(),
            match self {
                Topology::Single => "single".to_owned(),
                Topology::Cluster3 => "cluster3".to_owned(),
            },
        )
    }
}

pub struct CassandraBench {
    compression: Option<Compression>,
    topology: Topology,
}

impl CassandraBench {
    pub fn new(compression: Option<Compression>, topology: Topology) -> Self {
        CassandraBench {
            compression,
            topology,
        }
    }
}

#[async_trait]
impl Bench for CassandraBench {
    type CloudResourcesRequired = ();
    type CloudResources = ();
    fn tags(&self) -> HashMap<String, String> {
        [
            ("db".to_owned(), "cassandra".to_owned()),
            ("topology".to_owned(), "1".to_owned()),
            self.topology.to_tag(),
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

    async fn orchestrate_cloud(
        &self,
        _resources: (),
        _running_in_release: bool,
        _profiling: Profiling,
        _bench_parameters: BenchParameters,
    ) -> Result<()> {
        todo!()
    }

    async fn orchestrate_local(
        &self,
        _running_in_release: bool,
        _profiling: Profiling,
        parameters: BenchParameters,
    ) -> Result<()> {
        let _docker_compose =
            docker_compose("benches/windsock/config/cassandra-1-docker-compose.yaml");
        let address = "127.0.0.1:9042";

        self.execute_run(address, &parameters).await;

        Ok(())
    }

    async fn run_bencher(
        &self,
        _resources: &str,
        parameters: BenchParameters,
        reporter: UnboundedSender<Report>,
    ) {
        let session = Arc::new(
            SessionBuilder::new()
                .known_nodes(["172.16.1.2:9042"])
                // We do not need to refresh metadata as there is nothing else fiddling with the topology or schema.
                // By default the metadata refreshes every 60s and that can cause performance issues so we disable it by using an absurdly high refresh interval
                .cluster_metadata_refresh_interval(Duration::from_secs(10000000000))
                .user("cassandra", "cassandra")
                .compression(self.compression)
                .build()
                .await
                .unwrap(),
        );

        let tasks = BenchTaskCassandra { session }
            .spawn_tasks(reporter.clone(), parameters.operations_per_second)
            .await;

        let start = Instant::now();
        reporter.send(Report::Start).unwrap();

        for _ in 0..parameters.runtime_seconds {
            let second = Instant::now();
            tokio::time::sleep(Duration::from_secs(1)).await;
            reporter
                .send(Report::SecondPassed(second.elapsed()))
                .unwrap();
        }

        reporter.send(Report::FinishedIn(start.elapsed())).unwrap();

        // make sure the tasks complete before we drop the database they are connecting to
        for task in tasks {
            task.await.unwrap();
        }
    }
}

#[derive(Clone)]
struct BenchTaskCassandra {
    session: Arc<Session>,
}

#[async_trait]
impl BenchTask for BenchTaskCassandra {
    async fn run_one_operation(&self) -> Result<(), String> {
        self.session
            .query_unpaged("SELECT * FROM system.peers", ())
            .await
            .map_err(|err| format!("{err:?}"))
            .map(|_| ())
    }
}
