use crate::{Configuration, Datapoint};
use click_benchmark::cpio::prepare_cpio_archive;
use click_benchmark::vm::{start_click, wait_until_driver_start, FileSystem, DATA_IFACE};
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::Duration;

pub fn measure_throughput(config: &Configuration) -> Vec<Datapoint> {
    println!("Preparing CPIO archive");
    let cpio = prepare_cpio_archive(
        &create_click_configuration(config),
        config.bpfilter_program.map(|(x, _)| x).map(PathBuf::from),
        config.bpfilter_program.map(|(_, x)| x).map(PathBuf::from),
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
    const READING_INTERVAL_MS: u64 = 200;

    let mut child = Command::new("ping")
        .stdout(Stdio::piped())
        .args(&[
            "-I",
            DATA_IFACE,
            "-i",
            &(READING_INTERVAL_MS as f64 / 1000.0).to_string(),
            "172.44.0.2"
        ])
        .spawn()
        .expect("couldn't start ping");

    let stdout = BufReader::new(child.stdout.take().expect("couldn't get stdout"));

    // warmup for 3 seconds
    const WARMUP_TIME_MS: u64 = 3_000;
    const WARMUP_LINES: usize = (WARMUP_TIME_MS / READING_INTERVAL_MS) as usize;

    println!("Warming up for {:.2}s", WARMUP_TIME_MS as f64 / 1000.0);
    let lines = stdout.lines().skip(WARMUP_LINES);

    // read for 60 seconds
    const MEASUREMENT_TIME: u64 = 60_000;
    const MEASUREMENT_LINES: usize = (MEASUREMENT_TIME / READING_INTERVAL_MS) as usize;

    println!("Measuring for {:.2}s", MEASUREMENT_TIME as f64 / 1000.0);
    let mut datapoints = Vec::with_capacity(MEASUREMENT_LINES);
    for (idx, line) in lines.enumerate() {
        if idx >= MEASUREMENT_LINES {
            break;
        }

        let line = line.expect("couldn't read line");
        let latency = parse_ping_output(&line);

        let datapoint = Datapoint {
            time: (idx as u64 + 1) * READING_INTERVAL_MS,
            latency,
        };
        datapoints.push(datapoint);

        println!(
            "{:.2}s / {}s - current latency: {} ns",
            datapoint.time as f64 / 1000.0,
            MEASUREMENT_TIME as f64 / 1000.0,
            datapoint.latency.as_nanos(),
        );
    }

    // cleanup
    child.kill().expect("couldn't kill ping");

    datapoints
}

fn parse_ping_output(line: &str) -> Duration {
    let time_str = line.split_ascii_whitespace().nth_back(1).expect("couldn't get time");
    let time = time_str.strip_prefix("time=").expect("couldn't strip prefix");
    let latency: f64 = time.parse().expect("couldn't parse time");

    Duration::try_from_secs_f64(latency / 1000.0).expect("couldn't convert to duration")
}

fn create_click_configuration(config: &Configuration) -> String {
    let click_element_config = config.click_config.unwrap_or("");

    format!(
        r#"
FromDevice(0)
 -> c1 :: Classifier(12/0806 20/0001,
                     12/0800);

c1[0] -> ARPResponder(172.44.0.2 $MAC0)
      -> ToDevice(0);

c1[1] -> CheckIPHeader(14)
 {click_element_config}
 -> ICMPPingResponder()
 -> EtherMirror()
 -> ToDevice(0);
"#
    )
}
