// Module responsible for handling *throughput* benchmarks.
//
// # Preparation
// 1. Start click VM with the desired configuration
// 2. Wait until the router is ready
//
// # Benchmarking procedure
// 1. The Click router generates a lot of traffic by itself
// 2. The generated packets pass through the specified element (or specifically not)
// 3. The traffic is sent to the host machine
// 4. The host machine measures the throughput using bmon

mod measurement;
mod persistence;
mod plots;
mod statistics;
mod summary;

use crate::measurement::measure_throughput;
use crate::persistence::{dump_measurement, dump_statistics, dump_summary, restore_measurement};
use serde::{Deserialize, Serialize};
use std::fs::OpenOptions;
use crate::summary::calculate_summary;

struct Configuration<'a> {
    name: &'a str,
    bpfilter_program: Option<&'a str>,
    click_config: Option<&'a str>,
}

const CONFIGURATIONS: &[Configuration] = &[
    Configuration {
        name: "baseline",
        bpfilter_program: None,
        click_config: None,
    },
    Configuration {
        name: "pass (IPFilter)",
        bpfilter_program: None,
        click_config: Some("-> IPFilter(allow all) "),
    },
    Configuration {
        name: "pass (bpfilter)",
        bpfilter_program: Some("bpfilters/pass"),
        click_config: Some("-> BPFilter(ID 1, FILE pass) "),
    },
    Configuration {
        name: "pass (bpfilter - JIT)",
        bpfilter_program: Some("bpfilters/pass"),
        click_config: Some("-> BPFilter(ID 1, FILE pass, JIT true) "),
    },
    Configuration {
        name: "target port (IPFilter)",
        bpfilter_program: None,
        click_config: Some("-> IPFilter(deny dst port 1234, allow all) "),
    },
    Configuration {
        name: "target port (bpfilter)",
        bpfilter_program: Some("bpfilters/target-port"),
        click_config: Some("-> BPFilter(ID 1, FILE target-port) "),
    },
    Configuration {
        name: "target port (bpfilter - JIT)",
        bpfilter_program: Some("bpfilters/target-port"),
        click_config: Some("-> BPFilter(ID 1, FILE target-port, JIT true) "),
    },
];

#[derive(Default, Copy, Clone, Debug, Serialize, Deserialize)]
pub struct Datapoint {
    time: u64,
    rx_packets: u64,
    rx_bytes: u64,
}

pub fn main() {
    let skip_measurement = std::env::args()
        .nth(1)
        .map(|arg| arg == "--skip-measurement")
        .unwrap_or(false);
    let except = std::env::args().skip(1)
        .filter(|arg| arg.starts_with("--except"))
        .map(|arg| arg.split('=').nth(1).unwrap().to_string())
        .collect::<Vec<String>>();

    let mut datapoints_per_config = Vec::with_capacity(CONFIGURATIONS.len());
    for config in CONFIGURATIONS.iter() {
        let datapoints = run_benchmark(config, skip_measurement, &except);
        datapoints_per_config.push((config.name, datapoints));
    }

    let summary = calculate_summary(&datapoints_per_config);
    dump_summary(&summary);
    println!("\n=== Summary ===\n{summary}");

    plots::whisker().wait().expect("whisker failed");
}

fn run_benchmark(config: &Configuration, skip_measurement: bool, except: &[String]) -> Vec<Datapoint> {
    println!("\n=== Running benchmark for {} ===", config.name);

    let datapoints = if !skip_measurement || except.contains(&config.name.to_string()) {
        let datapoints = measure_throughput(config);
        dump_measurement(config.name, &datapoints);

        datapoints
    } else {
        restore_measurement(config.name)
    };

    // calculate statistics
    let statistics = statistics::calculate_statistics(&datapoints);
    dump_statistics(config.name, &statistics);
    println!("\n{statistics}");

    // plot
    plots::create_plots(config.name, &datapoints);

    datapoints
}
