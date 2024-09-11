use std::process::{Child, Command};

pub fn whisker_pps() -> Child {
    const PLOT_WHISKER_PATH: &str = "benches/throughput/plots/plot_whisker_pps.py";
    const SUMMARY_PATH: &str = "target/throughput/summary.json";
    const OUTPUT_PATH: &str = "target/throughput/summary-pps.png";

    Command::new("python3")
        .args(&[
            PLOT_WHISKER_PATH,
            SUMMARY_PATH,
            "-o",
            OUTPUT_PATH,
        ])
        .spawn()
        .expect("failed to spawn python plotting process")
}

pub fn whisker_bps() -> Child {
    const PLOT_WHISKER_PATH: &str = "benches/throughput/plots/plot_whisker_bps.py";
    const SUMMARY_PATH: &str = "target/throughput/summary.json";
    const OUTPUT_PATH: &str = "target/throughput/summary-bps.png";

    Command::new("python3")
        .args(&[
            PLOT_WHISKER_PATH,
            SUMMARY_PATH,
            "-o",
            OUTPUT_PATH,
        ])
        .spawn()
        .expect("failed to spawn python plotting process")
}
