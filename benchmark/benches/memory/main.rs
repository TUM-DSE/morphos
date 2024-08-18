// Module responsible for handling *memory* benchmarks.
//
// # Benchmarking procedure
// 1. Start click VM with the desired configuration
// 2. Wait until the router is ready
// 3. Measure the memory usage of the click VM
//
// We do not need to generate traffic as the memory usage is not dependent on the traffic.

mod measurement;
mod persistence;
mod plots;
mod statistics;
mod summary;

use crate::measurement::measure_memory_usage;
use crate::persistence::{dump_measurement, dump_statistics, dump_summary, restore_measurement};
use crate::summary::calculate_summary;

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
        name: "pass (IPFilter)",
        bpfilter_program: None,
        click_config: Some("-> IPFilter(allow all) "),
    },
    Configuration {
        name: "pass (bpfilter)",
        bpfilter_program: Some(("bpfilters/pass", "bpfilters/pass.sig")),
        click_config: Some("-> BPFilter(ID 1, FILE pass, SIGNATURE pass.sig) "),
    },
    Configuration {
        name: "pass (bpfilter - JIT)",
        bpfilter_program: Some(("bpfilters/pass", "bpfilters/pass.sig")),
        click_config: Some("-> BPFilter(ID 1, FILE pass, SIGNATURE pass.sig, JIT true) "),
    },
    Configuration {
        name: "2x pass (bpfilter)",
        bpfilter_program: Some(("bpfilters/pass", "bpfilters/pass.sig")),
        click_config: Some("-> BPFilter(ID 1, FILE pass, SIGNATURE pass.sig) -> BPFilter(ID 2, FILE pass, SIGNATURE pass.sig) "),
    },
    Configuration {
        name: "2x pass (bpfilter - JIT)",
        bpfilter_program: Some(("bpfilters/pass", "bpfilters/pass.sig")),
        click_config: Some("-> BPFilter(ID 1, FILE pass, SIGNATURE pass.sig, JIT true) -> BPFilter(ID 2, FILE pass, SIGNATURE pass.sig, JIT true) "),
    },
    Configuration {
        name: "3x pass (bpfilter)",
        bpfilter_program: Some(("bpfilters/pass", "bpfilters/pass.sig")),
        click_config: Some("-> BPFilter(ID 1, FILE pass, SIGNATURE pass.sig) -> BPFilter(ID 2, FILE pass, SIGNATURE pass.sig) -> BPFilter(ID 3, FILE pass, SIGNATURE pass.sig) "),
    },
    Configuration {
        name: "3x pass (bpfilter - JIT)",
        bpfilter_program: Some(("bpfilters/pass", "bpfilters/pass.sig")),
        click_config: Some("-> BPFilter(ID 1, FILE pass, SIGNATURE pass.sig, JIT true) -> BPFilter(ID 2, FILE pass, SIGNATURE pass.sig, JIT true) -> BPFilter(ID 3, FILE pass, SIGNATURE pass.sig, JIT true) "),
    },
];

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

fn run_benchmark(config: &Configuration, skip_measurement: bool, except: &[String]) -> Vec<u64> {
    println!("\n=== Running benchmark for {} ===", config.name);

    let datapoints = if !skip_measurement || except.contains(&config.name.to_string()) {
        let datapoints = measure_memory_usage(config);
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
