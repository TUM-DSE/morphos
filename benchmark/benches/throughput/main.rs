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
use crate::summary::calculate_summary;
use serde::{Deserialize, Serialize};

struct Configuration<'a> {
    name: &'a str,
    files: &'a [&'a str],
    click_config: Option<&'a str>,
}

const CONFIGURATIONS: &[Configuration] = &[
    // === baseline ===
    Configuration {
        name: "baseline",
        files: &[],
        click_config: Some("-> encap_then_out; "),
    },
    // === target-port ===
    Configuration {
        name: "target-port (IPFilter)",
        files: &[],
        click_config: Some("-> IPFilter(deny dst port 1234, allow all) -> encap_then_out; "),
    },
    Configuration {
        name: "target-port (BPFFilter)",
        files: &["bpfilters/target-port", "bpfilters/target-port.sig"],
        click_config: Some("-> BPFilter(ID 1, FILE target-port, SIGNATURE target-port.sig) -> encap_then_out; "),
    },
    Configuration {
        name: "target-port (BPFFilter - JIT)",
        files: &["bpfilters/target-port", "bpfilters/target-port.sig"],
        click_config: Some("-> BPFilter(ID 1, FILE target-port, SIGNATURE target-port.sig, JIT true) -> encap_then_out; "),
    },
    // === Round Robin ===
    Configuration {
        name: "round-robin (RoundRobinSwitch)",
        files: &[],
        click_config: Some("-> rr :: RoundRobinSwitch; rr[0] -> encap_then_out; rr[1] -> encap_then_out; "),
    },
    Configuration {
        name: "round-robin (BPFClassifier)",
        files: &["bpfilters/round-robin", "bpfilters/round-robin.sig"],
        click_config: Some("-> rr :: BPFClassifier(ID 1, FILE round-robin, SIGNATURE round-robin.sig); rr[0] -> encap_then_out; rr[1] -> encap_then_out; "),
    },
    Configuration {
        name: "round-robin (BPFClassifier - JIT)",
        files: &["bpfilters/round-robin", "bpfilters/round-robin.sig"],
        click_config: Some("-> rr :: BPFClassifier(ID 1, FILE round-robin, SIGNATURE round-robin.sig, JIT true); rr[0] -> encap_then_out; rr[1] -> encap_then_out; "),
    },
    // === Strip Ether VLAN Header ===
    Configuration {
        name: "strip-ether-vlan-header (StripEtherVLANHeader)",
        files: &[],
        click_config: Some("-> EtherEncap(0x800, $MAC0, $MAC0) -> StripEtherVLANHeader -> encap_then_out; "),
    },
    Configuration {
        name: "strip-ether-vlan-header (BPFFilter)",
        files: &["bpfilters/strip-ether-vlan-header", "bpfilters/strip-ether-vlan-header.sig"],
        click_config: Some("-> EtherEncap(0x800, $MAC0, $MAC0) -> BPFRewriter(ID 1, FILE strip-ether-vlan-header, SIGNATURE strip-ether-vlan-header.sig) -> encap_then_out; "),
    },
    Configuration {
        name: "strip-ether-vlan-header (BPFFilter - JIT)",
        files: &["bpfilters/strip-ether-vlan-header", "bpfilters/strip-ether-vlan-header.sig"],
        click_config: Some("-> EtherEncap(0x800, $MAC0, $MAC0) -> BPFRewriter(ID 1, FILE strip-ether-vlan-header, SIGNATURE strip-ether-vlan-header.sig, JIT true) -> encap_then_out; "),
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
    let except = std::env::args()
        .skip(1)
        .filter(|arg| arg.starts_with("--except"))
        .map(|arg| arg.split('=').nth(1).unwrap().to_string())
        .collect::<Vec<String>>();
    let only = std::env::args()
        .skip(1)
        .filter(|arg| arg.starts_with("--only"))
        .map(|arg| arg.split('=').nth(1).unwrap().to_string())
        .collect::<Vec<String>>();

    let mut configs: Vec<&Configuration> = CONFIGURATIONS.iter().collect();
    if only.len() > 0 {
        let new_configs: Vec<&Configuration> = CONFIGURATIONS
            .iter()
            .filter(|config| only.contains(&config.name.to_string()))
            .collect();
        configs = new_configs;
    }
    let mut datapoints_per_config = Vec::with_capacity(configs.len());
    for config in configs.iter() {
        let datapoints = run_benchmark(config, skip_measurement, &except);
        datapoints_per_config.push((config.name, datapoints));
    }

    let summary = calculate_summary(&datapoints_per_config);
    dump_summary(&summary);
    println!("\n=== Summary ===\n{summary}");

    plots::whisker_pps().wait().expect("whisker pps failed");
    plots::whisker_bps().wait().expect("whisker bps failed");
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
