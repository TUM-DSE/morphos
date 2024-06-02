mod measurement;
mod persistence;
mod plots;
mod statistics;

use crate::measurement::measure_throughput;
use crate::persistence::{dump_measurement, dump_statistics, restore_measurement};
use serde::{Deserialize, Serialize};
use std::fs::OpenOptions;

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
        click_config: Some("-> IPFilter(allow all)"),
    },
    Configuration {
        name: "pass (bpfilter)",
        bpfilter_program: Some("bpfilters/pass"),
        click_config: Some("-> BPFilter(ID 1, FILE pass)"),
    },
    Configuration {
        name: "source port (IPFilter)",
        bpfilter_program: None,
        click_config: Some("-> IPFilter(deny src port 1234, allow all)"),
    },
    Configuration {
        name: "target port (bpfilter)",
        bpfilter_program: Some("bpfilters/target-port"),
        click_config: Some("-> BPFilter(ID 1, FILE target-port)"),
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

    for config in CONFIGURATIONS.iter() {
        run_benchmark(config, skip_measurement);
    }
}

fn run_benchmark(config: &Configuration, skip_measurement: bool) {
    println!("=== Running benchmark for {} ===", config.name);

    let datapoints = if !skip_measurement {
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
}
