# Windsock - A DB benchmarking framework

[![Crates.io](https://img.shields.io/crates/v/windsock.svg)](https://crates.io/crates/windsock)
[![Docs](https://docs.rs/windsock/badge.svg)](https://docs.rs/windsock)
[![dependency status](https://deps.rs/repo/github/shotover/windsock/status.svg)](https://deps.rs/repo/github/shotover/windsock)

<p align="left">
  <img width="500px" alt="Shotover logo" src="https://github.com/shotover/windsock/blob/example_image/example.png">
</p>

Windsock is suitable for:

* Iteratively testing performance during development of a database or service (use a different tool for microbenchmarks)
* Investigating performance of different workloads on a database you intend to use.

What you do:

* Bring your own rust async compatible DB driver
* Define your benchmark logic which reports some simple stats back to windsock
* Define your pool of benchmarks

What windsock does:

* Provides a CLI from which you can:
  * Query available benchmarks
  * Run benchmarks matching specific tags.
    * windsock can automatically or manually setup and cleanup required cloud resources
  * Process benchmark results into readable tables
    * Baselines can be set and then compared against

## Add windsock benches to your project

### 1

Import windsock and setup a cargo bench for windsock:

```toml
[dev-dependencies]
windsock = "0.1"

[[bench]]
name = "windsock"
harness = false
```

All windsock benchmarks should go into this one bench.

### 2

Setup a shortcut to run windsock in `.cargo/config.toml`:

```toml
[alias]
windsock = "test --release --bench windsock --"
windsock-debug = "test --bench windsock --"
```

This allows us to run `cargo windsock` instead of `cargo --test --release --bench windsock --`.

### 3

Then at `benches/windsock` create a benchmark like this (simplified):

```rust
fn main() {
    // Define our benchmarks and give them to windsock
    Windsock::new(vec![
        Box::new(CassandraBench { topology: Topology::Cluster3 }),
        Box::new(CassandraBench { topology: Topology::Single })
    ])
    // Hand control of the app over to windsock
    // Windsock processes CLI args, possibly running benchmarks and then terminates.
    .run();
}

pub struct CassandraBench { topology: Topology }

#[async_trait]
impl Bench for CassandraBench {
    // define tags that windsock will use to filter and name the benchmark instance
    fn tags(&self) -> HashMap<String, String> {
        [
            ("name".to_owned(), "cassandra".to_owned()),
            (
                "topology".to_owned(),
                match &self.topology {
                    Topology::Single => "single".to_owned(),
                    Topology::Cluster3 => "cluster3".to_owned(),
                },
            ),
        ]
        .into_iter()
        .collect()
    }

    // the benchmark logic for this benchmark instance
    async fn run(&self, runtime_seconds: usize, operations_per_second: Option<u64>, reporter: UnboundedSender<Report>) {
        // bring up the DB
        let _handle = init_cassandra();

        // create the DB driver session
        let session = init_session().await;

        // spawn tokio tasks to concurrently hit the database
        // The exact query is defined in `run_one_operation` below
        BenchTaskCassandra { session }.spawn_tasks(reporter.clone(), operations_per_second).await;

        // tell windsock to begin benchmarking
        reporter.send(Report::Start).unwrap();
        let start = Instant::now();

        // run the bench for the time requested by the user on the CLI (defaults to 15s)
        tokio::time::sleep(Duration::from_secs(runtime_seconds)).await;

        // tell windsock to finalize the benchmark
        reporter.send(Report::FinishedIn(start.elapsed())).unwrap();
    }
}

// This struct is cloned once for each tokio task it will be run in.
#[derive(Clone)]
struct BenchTaskCassandra {
    session: Arc<Session>,
}

#[async_trait]
impl BenchTask for BenchTaskCassandra {
    async fn run_one_operation(&self) -> Result<(), String> {
        self.session.query("SELECT * FROM table").await
    }
}
```

This example is simplified for demonstration purposes, refer to `windsock/benches/windsock` in this repo for a full working example.

## How to perform various tasks in `cargo windsock` CLI

### Just run every bench

```shell
> cargo windsock run-local
```

### Run benches with matching tags and view all the results in one table

```shell
> cargo windsock run-local db=kafka OPS=1000 topology=single # run benchmarks matching some tags
> cargo windsock results # view the results of the benchmarks with the same tags in a single table
```

### Iteratively compare results against a previous implementation

```shell
> git checkout main # checkout original implementation
> cargo windsock run-local # run all benchmarks
> cargo windsock baseline-set # set the last benchmark run as the baseline
> vim src/main.rs # modify implementation
> cargo windsock run-local # run all benchmarks, every result is compared against the baseline
> cargo windsock results # view those results in a nice table
> vim src/main.rs # modify implementation again
> cargo windsock run-local # run all benchmarks, every result is compared against the baseline
```

### Run benchmarks in the cloud (simple)

```shell
# create cloud resources, run benchmarks and then cleanup - all in one command
> cargo windsock cloud-setup-run-cleanup
```

### Iteratively compare results against a previous implementation (running in a remote cloud)

```shell
# Setup the cloud resources and then form a baseline
> git checkout main # checkout original implementation
> cargo windsock cloud-setup db=kafka # setup the cloud resources required to run all kafka benchmarks
> cargo windsock cloud-run db=kafka # run all the kafka benchmarks in the cloud
> cargo windsock baseline-set # set the last benchmark run as the baseline

# Make a change and and measure the effect
> vim src/main.rs # modify implementation
> cargo windsock cloud-run db=kafka # run all benchmarks, every result is compared against the baseline
> cargo windsock results # view those results in a nice table, compared against the baseline

# And again
> vim src/main.rs # modify implementation again
> cargo windsock cloud-run db=kafka # run all benchmarks, every result is compared against the baseline

# And finally...
> cargo windsock cloud-cleanup # Terminate all the cloud resources now that we are done
```

### Generate graph webpage

TODO: planned, but not implemented

```shell
> cargo windsock local-run # run all benches
> cargo windsock generate-webpage # generate a webpage from the results
```
