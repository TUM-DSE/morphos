// Module responsible for handling *live reconfiguration* benchmarks.
//
// # Preparation
// 1. Start click VM with the desired configuration
// 2. Wait until the router is ready
//
// # Benchmarking procedure
// 1. Triggers reconfiguration
// 2. Waits until "Reconfiguring BPFilter..." is printed => starts measurement from here (this is the start of the "downtime" of the router"
// 3. Waits until "Reconfigured BPFilter" is printed = ends measurement from here (from here on, the router is available again)

use std::cell::RefCell;
use std::fmt::format;
use std::io::{BufRead, BufReader, Lines};
use std::net::UdpSocket;
use std::path::PathBuf;
use std::process::ChildStdout;
use std::time::{Duration, Instant};

use anyhow::Context;
use click_benchmark::cpio::prepare_cpio_archive;
use click_benchmark::vm::{self, wait_until_ready, FileSystem, CONTROL_ADDR};
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};

struct Configuration<'a> {
    name: &'a str,
    bpfilter_program: &'a str,
    jit: bool,
}

const CONFIGURATIONS: &[Configuration] = &[
    Configuration {
        name: "pass",
        bpfilter_program: "pass",
        jit: false,
    },
    Configuration {
        name: "pass-jit",
        bpfilter_program: "pass",
        jit: true,
    },
    Configuration {
        name: "drop",
        bpfilter_program: "drop",
        jit: false,
    },
    Configuration {
        name: "drop-jit",
        bpfilter_program: "drop",
        jit: true,
    },
    Configuration {
        name: "target-port",
        bpfilter_program: "target-port",
        jit: false,
    },
    Configuration {
        name: "target-port-jit",
        bpfilter_program: "target-port",
        jit: true,
    },
];

const BPFILTER_BASE_PATH: &str = "bpfilters";

pub fn live_reconfigure(c: &mut Criterion) {
    let mut group = c.benchmark_group("Live Reconfigure");

    for config in CONFIGURATIONS {
        // prepare click VM
        let cpio = prepare_cpio_archive(
            &create_click_configuration(config.bpfilter_program, config.jit),
            Some(&PathBuf::from(BPFILTER_BASE_PATH).join(config.bpfilter_program)),
        )
            .expect("couldn't prepare cpio archive");

        let mut click_vm = vm::start_click(
            FileSystem::CpioArchive(&cpio.path.to_string_lossy()),
            &[
                "-netdev".to_string(),
                "bridge,id=en1,br=controlnet".to_string(),
                "-device".to_string(),
                "virtio-net-pci,netdev=en1".to_string(),
            ],
        )
            .expect("couldn't start click");

        // wait until the router is ready
        let mut lines = click_vm.stdout.take().unwrap().lines();
        wait_until_ready(&mut lines);

        let lines = RefCell::new(lines);

        group.bench_with_input(
            BenchmarkId::from_parameter(config.name),
            config,
            |b, config| {
                b.iter_custom(|iters| {
                    let mut sum = Duration::new(0, 0);
                    for _ in 0..iters {
                        let mut lines = lines.borrow_mut();
                        sum += run_benchmark(config, &mut *lines);
                    }
                    sum
                });
            },
        );
    }

    group.finish();
}

fn run_benchmark(config: &Configuration, lines: &mut Lines<BufReader<ChildStdout>>) -> Duration {
    trigger_reconfiguration(config.bpfilter_program).expect("couldn't trigger reconfiguration");
    wait_until_reconfiguration_start(lines);

    let now = Instant::now();
    wait_until_reconfiguration_end(lines);

    now.elapsed()
}

fn trigger_reconfiguration(program: &str) -> anyhow::Result<()> {
    let signature = format!("{program}.sig");

    let mut data = Vec::new();
    data.extend_from_slice(b"control");
    data.extend_from_slice(&1u64.to_le_bytes());
    data.extend_from_slice(&(program.len() as u64).to_le_bytes());
    data.extend_from_slice(program.as_bytes());
    data.extend_from_slice(&(signature.len() as u64).to_le_bytes());
    data.extend_from_slice(signature.as_bytes());

    let socket = UdpSocket::bind("0.0.0.0:0").context("couldn't bind to control addr")?;
    socket
        .send_to(&data, CONTROL_ADDR)
        .context("couldn't send packet")?;

    Ok(())
}

fn wait_until_reconfiguration_start(lines: &mut Lines<BufReader<ChildStdout>>) {
    lines
        .filter_map(Result::ok)
        .find(|line| line.contains("Reconfiguring BPFilter"));
}

fn wait_until_reconfiguration_end(lines: &mut Lines<BufReader<ChildStdout>>) {
    lines
        .filter_map(Result::ok)
        .find(|line| line.contains("Reconfigured BPFilter"));
}

fn create_click_configuration(bpfilter_program: &str, jit: bool) -> String {
    format!(
        r#"
// === Control network ===
elementclass ControlReceiver {{ $deviceid |
    FromDevice($deviceid)
     -> c0 :: Classifier(12/0806 20/0001,
                         12/0800,
                         -);

    // Answer ARP requests
    c0[0] -> ARPResponder(173.44.0.2 $MAC1)
          -> ToDevice($deviceid);

    // Handle IP packets
    c0[1] -> StripEtherVLANHeader
     -> CheckIPHeader
     -> IPFilter(allow dst port 4444, deny all)
     -> IPReassembler
     -> SetUDPChecksum
     -> CheckUDPHeader
     -> Control;

    c0[2] -> Discard;
}}

ControlReceiver(1);

// === Data network ===
FromDevice(0) -> Print('Received packet') -> BPFilter(ID 1, FILE {bpfilter_program}, JIT {jit_arg}) -> Discard;
"#, jit_arg = if jit { "true" } else { "false" }
    )
}

criterion_group!(benches, live_reconfigure);
criterion_main!(benches);
