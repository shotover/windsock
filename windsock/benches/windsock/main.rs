use cassandra::{CassandraBench, Topology};
use scylla::frame::Compression;
use windsock::cloud::NoCloud;
use windsock::{Bench, Windsock};
mod cassandra;
mod docker_compose;

fn main() {
    Windsock::new(
        itertools::iproduct!(
            [Some(Compression::Lz4), None],
            [Topology::Single, Topology::Cluster3]
        )
        .map(|(compression, topology)| {
            Box::new(CassandraBench::new(compression, topology)) as BoxedBench
        })
        .collect(),
        NoCloud::new_boxed(),
        &["release"],
    )
    .run();
}

pub type BoxedBench = Box<dyn Bench<CloudResourcesRequired = (), CloudResources = ()>>;
