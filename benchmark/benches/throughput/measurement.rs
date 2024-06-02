use crate::{Configuration, Datapoint};
use click_benchmark::cpio::prepare_cpio_archive;
use click_benchmark::vm::{start_click, wait_until_driver_start, FileSystem, DATA_IFACE};
use mac_address::mac_address_by_name;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Command, Stdio};

pub fn measure_throughput(config: &Configuration) -> Vec<Datapoint> {
    println!("Preparing CPIO archive");
    let cpio = prepare_cpio_archive(
        &create_click_configuration(config),
        config.bpfilter_program.map(PathBuf::from),
    )
    .expect("couldn't prepare cpio archive");

    println!("Starting Click");
    let mut click_vm = start_click(FileSystem::CpioArchive(&cpio.path.to_string_lossy()), &[])
        .expect("couldn't start click");

    wait_until_driver_start(&mut click_vm.stdout.take().unwrap().lines());
    println!("Click started and router ready");

    measure_datapoints()
}

fn measure_datapoints() -> Vec<Datapoint> {
    const READING_INTERVAL_MS: u64 = 100;

    let mut child = Command::new("bmon")
        .stdout(Stdio::piped())
        .args(&[
            "-p",
            DATA_IFACE,
            "-o",
            "format:fmt=$(attr:rx:packets) $(attr:rx:bytes)\n",
            "-r",
            &(READING_INTERVAL_MS as f64 / 1000.0).to_string(),
            "-R",
            "1",
        ])
        .spawn()
        .expect("couldn't start bmon");

    let stdout = BufReader::new(child.stdout.take().expect("couldn't get stdout"));

    // warmup for 3 seconds
    const WARMUP_TIME_MS: u64 = 3_000;
    const WARMUP_LINES: usize = (WARMUP_TIME_MS / READING_INTERVAL_MS) as usize;

    println!("Warming up for {:.2}s", WARMUP_TIME_MS as f64 / 1000.0);
    let mut lines = stdout.lines().skip(WARMUP_LINES - 1);

    // use the first line as baseline
    let baseline_line = lines
        .next()
        .expect("couldn't read line")
        .expect("couldn't read line");
    let (baseline_rx_packets, baseline_rx_bytes) = parse_bmon_output(&baseline_line);

    // read for 20 seconds
    const MEASUREMENT_TIME: u64 = 20_000;
    const MEASUREMENT_LINES: usize = (MEASUREMENT_TIME / READING_INTERVAL_MS) as usize;

    println!("Measuring for {:.2}s", MEASUREMENT_TIME as f64 / 1000.0);
    let mut datapoints = Vec::with_capacity(MEASUREMENT_LINES);
    for (idx, line) in lines.enumerate() {
        if idx >= MEASUREMENT_LINES {
            break;
        }

        let line = line.expect("couldn't read line");
        let (rx_packets, rx_bytes) = parse_bmon_output(&line);

        let datapoint = Datapoint {
            time: (idx as u64 + 1) * READING_INTERVAL_MS,
            rx_packets: rx_packets - baseline_rx_packets,
            rx_bytes: rx_bytes - baseline_rx_bytes,
        };
        datapoints.push(datapoint);

        println!(
            "{:.2}s / {}s - current rx packets: {}, rx bytes: {}",
            datapoint.time as f64 / 1000.0,
            MEASUREMENT_TIME as f64 / 1000.0,
            datapoint.rx_packets,
            datapoint.rx_bytes
        );
    }

    // cleanup
    child.kill().expect("couldn't kill bmon");

    datapoints
}

fn parse_bmon_output(line: &str) -> (u64, u64) {
    let mut parts = line.split_ascii_whitespace();
    let rx: u64 = parts
        .next()
        .expect("couldn't get rx")
        .parse()
        .expect("couldn't parse rx");
    let bytes: u64 = parts
        .next()
        .expect("couldn't get bytes")
        .parse()
        .expect("couldn't parse bytes");

    (rx, bytes)
}

fn create_click_configuration(config: &Configuration) -> String {
    let mac_address = mac_address_by_name(DATA_IFACE)
        .expect("couldn't get mac address")
        .expect("no mac address found");

    let click_element_config = config.click_config.unwrap_or("");

    format!(
        r#"
// need this to initialize the device 0
FromDevice(0) -> Discard;

InfiniteSource(DATA \<0800>, LENGTH 60, LIMIT -1, BURST 100000)
-> UDPIPEncap(172.44.0.2, 5678, 172.44.0.1, 5678)
-> EtherEncap(0x0800, $MAC0, {mac_address})
{click_element_config}
-> ToDevice(0);
"#
    )
}
