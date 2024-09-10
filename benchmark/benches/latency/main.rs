// Module responsible for handling *latency* benchmarks.
//
// # Preparation
// 1. Start click VM with the desired configuration
// 2. Wait until the router is ready
//
// # Benchmarking procedure
// 1. The host machine repeatedly sends ICMP ping packets to the click VM
// 2. The click VM responds to the ICMP packets
// 3. The host machine measures the latency of the ICMP packets

mod measurement;
mod persistence;
mod plots;
mod statistics;
mod summary;

use crate::measurement::measure_throughput;
use crate::persistence::{dump_measurement, dump_statistics, dump_summary, restore_measurement};
use crate::summary::calculate_summary;
use serde::{Deserialize, Serialize};
use std::time::Duration;

struct Configuration<'a> {
    name: &'a str,
    bpfilter_program: Option<(&'a str, &'a str)>,
    click_config: Option<&'a str>,
}

const CONFIGURATIONS: &[Configuration] = &[
    Configuration {
        name: "baseline",
        bpfilter_program: None,
        click_config: None,
    },
    Configuration {
        name: "1 element (no JIT)",
        bpfilter_program: Some(("bpfilters/pass", "bpfilters/pass.sig")),
        click_config: Some("-> BPFilter(ID 1, FILE pass, SIGNATURE pass.sig, JIT false) "),
    },
    Configuration {
        name: "1 element (JIT)",
        bpfilter_program: Some(("bpfilters/pass", "bpfilters/pass.sig")),
        click_config: Some("-> BPFilter(ID 1, FILE pass, SIGNATURE pass.sig, JIT true) "),
    },
    Configuration {
        name: "10 elements (no JIT)",
        bpfilter_program: Some(("bpfilters/pass", "bpfilters/pass.sig")),
        click_config: Some(
            r#"
        -> BPFilter(ID 1, FILE pass, SIGNATURE pass.sig, JIT false)
        -> BPFilter(ID 2, FILE pass, SIGNATURE pass.sig, JIT false)
        -> BPFilter(ID 3, FILE pass, SIGNATURE pass.sig, JIT false)
        -> BPFilter(ID 4, FILE pass, SIGNATURE pass.sig, JIT false)
        -> BPFilter(ID 5, FILE pass, SIGNATURE pass.sig, JIT false)
        -> BPFilter(ID 6, FILE pass, SIGNATURE pass.sig, JIT false)
        -> BPFilter(ID 7, FILE pass, SIGNATURE pass.sig, JIT false)
        -> BPFilter(ID 8, FILE pass, SIGNATURE pass.sig, JIT false)
        -> BPFilter(ID 9, FILE pass, SIGNATURE pass.sig, JIT false)
        -> BPFilter(ID 10, FILE pass, SIGNATURE pass.sig, JIT false)
        "#,
        ),
    },
    Configuration {
        name: "10 elements (JIT)",
        bpfilter_program: Some(("bpfilters/pass", "bpfilters/pass.sig")),
        click_config: Some(
            r#"
        -> BPFilter(ID 1, FILE pass, SIGNATURE pass.sig, JIT true)
        -> BPFilter(ID 2, FILE pass, SIGNATURE pass.sig, JIT true)
        -> BPFilter(ID 3, FILE pass, SIGNATURE pass.sig, JIT true)
        -> BPFilter(ID 4, FILE pass, SIGNATURE pass.sig, JIT true)
        -> BPFilter(ID 5, FILE pass, SIGNATURE pass.sig, JIT true)
        -> BPFilter(ID 6, FILE pass, SIGNATURE pass.sig, JIT true)
        -> BPFilter(ID 7, FILE pass, SIGNATURE pass.sig, JIT true)
        -> BPFilter(ID 8, FILE pass, SIGNATURE pass.sig, JIT true)
        -> BPFilter(ID 9, FILE pass, SIGNATURE pass.sig, JIT true)
        -> BPFilter(ID 10, FILE pass, SIGNATURE pass.sig, JIT true)
        "#,
        ),
    },
];

#[derive(Default, Copy, Clone, Debug, Serialize, Deserialize)]
pub struct Datapoint {
    time: u64,
    latency: Duration,
}

pub fn main() {
    let skip_measurement = std::env::args()
        .nth(1)
        .map(|arg| arg == "--skip-measurement")
        .unwrap_or(false);
    let except = std::env::args()
        .skip(1)
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

fn run_benchmark(
    config: &Configuration,
    skip_measurement: bool,
    except: &[String],
) -> Vec<Datapoint> {
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
