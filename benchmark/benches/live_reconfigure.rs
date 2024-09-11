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
    signature_file: &'a str,
    click_config: &'a str,
}

const CONFIGURATIONS: &[Configuration] = &[
    Configuration {
        name: "pass (BPFFilter)",
        bpfilter_program: "pass",
        signature_file: "pass.sig",
        click_config: "-> BPFilter(ID 1, FILE pass, SIGNATURE pass.sig, JIT false)",
    },
    Configuration {
        name: "pass (BPFFilter - JIT)",
        bpfilter_program: "pass",
        signature_file: "pass.sig",
        click_config: "-> BPFilter(ID 1, FILE pass, SIGNATURE pass.sig, JIT true)",
    },
    Configuration {
        name: "drop (BPFFilter)",
        bpfilter_program: "drop",
        signature_file: "drop.sig",
        click_config: "-> BPFilter(ID 1, FILE drop, SIGNATURE drop.sig, JIT false)",
    },
    Configuration {
        name: "drop (BPFFilter - JIT)",
        bpfilter_program: "drop",
        signature_file: "drop.sig",
        click_config: "-> BPFilter(ID 1, FILE drop, SIGNATURE drop.sig, JIT true)",
    },
    Configuration {
        name: "target-port (BPFFilter)",
        bpfilter_program: "target-port",
        signature_file: "target-port.sig",
        click_config: "-> BPFilter(ID 1, FILE target-port, SIGNATURE target-port.sig, JIT false)",
    },
    Configuration {
        name: "target-port (BPFFilter - JIT)",
        bpfilter_program: "target-port",
        signature_file: "target-port.sig",
        click_config: "-> BPFilter(ID 1, FILE target-port, SIGNATURE target-port.sig, JIT true)",
    },
    Configuration {
        name: "rate-limiter",
        bpfilter_program: "rate-limiter (BPFFilter)",
        signature_file: "rate-limiter.sig",
        click_config: "-> BPFilter(ID 1, FILE rate-limiter, SIGNATURE rate-limiter.sig, JIT false)",
    },
    Configuration {
        name: "rate-limiter (BPFFilter - JIT)",
        bpfilter_program: "rate-limiter",
        signature_file: "rate-limiter.sig",
        click_config: "-> BPFilter(ID 1, FILE rate-limiter, SIGNATURE rate-limiter.sig, JIT true)",
    },
    Configuration {
        name: "round-robin (BPFClassifier)",
        bpfilter_program: "round-robin",
        signature_file: "round-robin.sig",
        click_config: "-> BPFClassifier(ID 1, FILE round-robin, SIGNATURE round-robin.sig, JIT false)",
    },
    Configuration {
        name: "round-robin (BPFClassifier - JIT)",
        bpfilter_program: "round-robin",
        signature_file: "round-robin.sig",
        click_config: "-> BPFClassifier(ID 1, FILE round-robin, SIGNATURE round-robin.sig, JIT true)",
    },
    Configuration {
        name: "udp-tcp-classifier (BPFClassifier)",
        bpfilter_program: "udp-tcp-classifier",
        signature_file: "udp-tcp-classifier.sig",
        click_config: "-> BPFClassifier(ID 1, FILE udp-tcp-classifier, SIGNATURE udp-tcp-classifier.sig, JIT false)",
    },
    Configuration {
        name: "udp-tcp-classifier (BPFClassifier - JIT)",
        bpfilter_program: "udp-tcp-classifier",
        signature_file: "udp-tcp-classifier.sig",
        click_config: "-> BPFClassifier(ID 1, FILE udp-tcp-classifier, SIGNATURE udp-tcp-classifier.sig, JIT true)",
    },
    Configuration {
        name: "strip-ether-vlan-header (BPFRewriter)",
        bpfilter_program: "strip-ether-vlan-header",
        signature_file: "strip-ether-vlan-header.sig",
        click_config: "-> BPFRewriter(ID 1, FILE strip-ether-vlan-header, SIGNATURE strip-ether-vlan-header.sig, JIT false)",
    },
    Configuration {
        name: "strip-ether-vlan-header (BPFRewriter - JIT)",
        bpfilter_program: "strip-ether-vlan-header",
        signature_file: "strip-ether-vlan-header.sig",
        click_config: "-> BPFRewriter(ID 1, FILE strip-ether-vlan-header, SIGNATURE strip-ether-vlan-header.sig, JIT true)",
    },
];

const BPFILTER_BASE_PATH: &str = "bpfilters";

pub fn live_reconfigure(c: &mut Criterion) {
    let mut group = c.benchmark_group("live-reconfigure");

    for config in CONFIGURATIONS {
        // prepare click VM
        let cpio = prepare_cpio_archive(
            &create_click_configuration(config.click_config),
            &[PathBuf::from(BPFILTER_BASE_PATH).join(config.bpfilter_program), PathBuf::from(BPFILTER_BASE_PATH).join(config.signature_file)],
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
    trigger_reconfiguration(config.bpfilter_program, config.signature_file)
        .expect("couldn't trigger reconfiguration");
    wait_until_reconfiguration_start(lines);

    let now = Instant::now();
    wait_until_reconfiguration_end(lines);

    now.elapsed()
}

fn trigger_reconfiguration(program: &str, signature: &str) -> anyhow::Result<()> {
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
        .find(|line| line.contains("Reconfiguring "));
}

fn wait_until_reconfiguration_end(lines: &mut Lines<BufReader<ChildStdout>>) {
    lines
        .filter_map(Result::ok)
        .find(|line| line.contains("Reconfigured "));
}

fn create_click_configuration(click_config: &str) -> String {
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
FromDevice(0) -> Print('Received packet') {click_config} -> Discard;
"#
    )
}

criterion_group!(benches, live_reconfigure);
criterion_main!(benches);
