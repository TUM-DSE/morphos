use crate::statistics::Statistics;
use crate::Datapoint;
use std::path::PathBuf;

pub fn dump_measurement(name: &str, datapoints: &[Datapoint]) {
    let path = base_dir(name).join("measurement.json");

    let json = serde_json::to_string_pretty(&datapoints).expect("couldn't serialize datapoints");
    std::fs::write(path, json).expect("couldn't write datapoints");
}

pub fn restore_measurement(name: &str) -> Vec<Datapoint> {
    let path = base_dir(name).join("measurement.json");

    let json = std::fs::read_to_string(path).expect("couldn't read datapoints");
    serde_json::from_str(&json).expect("couldn't deserialize datapoints")
}

pub fn dump_statistics(name: &str, statistics: &Statistics) {
    let path = base_dir(name).join("statistics.json");

    let json = serde_json::to_string_pretty(statistics).expect("couldn't serialize summary");
    std::fs::write(path, json).expect("couldn't write summary");
}

pub fn plot_path(name: &str, plot_name: &str) -> PathBuf {
    base_dir(name).join(plot_name)
}

fn base_dir(name: &str) -> PathBuf {
    let dir = std::env::current_dir()
        .expect("couldn't get current dir")
        .join("target/throughput")
        .join(name);
    std::fs::create_dir_all(&dir).expect("couldn't create dir");

    dir
}
