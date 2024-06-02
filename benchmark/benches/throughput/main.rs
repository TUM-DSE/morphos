mod measurement;
mod persistence;
mod plots;
mod statistics;

use crate::measurement::measure_throughput;
use crate::persistence::{dump_measurement, dump_statistics, restore_measurement};
use serde::{Deserialize, Serialize};

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
    let name = "baseline";
    let datapoints = if !skip_measurement {
        let datapoints = measure_throughput();
        dump_measurement(name, &datapoints);

        datapoints
    } else {
        restore_measurement(name)
    };

    // calculate statistics
    let statistics = statistics::calculate_statistics(&datapoints);
    dump_statistics(name, &statistics);
    println!("\n{statistics}");

    // plot
    plots::create_plots(name, &datapoints);
}
