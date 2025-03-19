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
        name: "rate-limiter",
        bpfilter_program: "rate-limiter",
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

    // vnfs = [ "empty", "filter", "ids", "mirror", "nat", "firewall-2" ]
    Configuration {
        name: "empty-jit",
        bpfilter_program: "pass",
        signature_file: "pass.sig",
        click_config: "-> BPFilter(ID 1, FILE pass, SIGNATURE pass.sig, JIT true)",
    },
    Configuration {
        name: "filter-jit",
        bpfilter_program: "target-port",
        signature_file: "target-port.sig",
        click_config: "-> BPFilter(ID 1, FILE target-port, SIGNATURE target-port.sig, JIT true)",
    },
    Configuration {
        name: "ids-jit",
        bpfilter_program: "stringmatcher",
        signature_file: "stringmatcher.sig",
        click_config: "-> BPFilter(ID 1, FILE stringmatcher, SIGNATURE stringmatcher.sig, JIT true)",
    },
    Configuration {
        name: "mirror-jit",
        bpfilter_program: "ether-mirror",
        signature_file: "ether-mirror.sig",
        click_config: "-> BPFRewriter(ID 1, FILE ether-mirror, SIGNATURE ether-mirror.sig, JIT true)",
    },
    Configuration {
        name: "nat-jit",
        bpfilter_program: "nat",
        signature_file: "nat.sig",
        click_config: "-> BPFRewriter(ID 1, FILE nat, SIGNATURE nat.sig, JIT true)",
    },
    Configuration {
        name: "firewall-2-jit",
        bpfilter_program: "firewall-2",
        signature_file: "firewall-2.sig",
        click_config: "-> BPFRewriter(ID 1, FILE firewall-2, SIGNATURE firewall-2.sig, JIT true)",
    },
    Configuration {
        name: "firewall-10000-jit",
        bpfilter_program: "firewall-10000",
        signature_file: "firewall-10000.sig",
        click_config: "-> BPFRewriter(ID 1, FILE firewall-10000, SIGNATURE firewall-10000.sig, JIT true)",
    },
];

const BPFILTER_BASE_PATH: &str = "bpfilters";

pub fn live_reconfigure(c: &mut Criterion) {
    let only = match  std::env::var("ONLY") {
        Ok(val) => val.split(',').map(|s| s.to_string()).collect::<Vec<String>>(),
        Err(_) => vec![],
    };
    let mut configs: Vec<&Configuration> = CONFIGURATIONS.iter().collect();
    if only.len() > 0 {
        let new_configs: Vec<&Configuration> = CONFIGURATIONS
            .iter()
            .filter(|config| only.contains(&config.name.to_string()))
            .collect();
        configs = new_configs;
    }

    let mut qemu_out_args = match std::env::var("QEMU_OUT") {
        Ok(out_path) => vec![
            "-chardev".to_string(),
            format!("stdio,id=char0,mux=on,logfile={},signal=off", out_path),
            "-serial".to_string(),
            "chardev:char0".to_string(),
            "-mon".to_string(),
            "chardev=char0".to_string(),
        ],
        Err(_) => vec![],
    };

    let mut group = c.benchmark_group("live-reconfigure");

    for config in configs {
        // prepare click VM
        let cpio = prepare_cpio_archive(
            &create_click_configuration(config.click_config),
            &[PathBuf::from(BPFILTER_BASE_PATH).join(config.bpfilter_program), PathBuf::from(BPFILTER_BASE_PATH).join(config.signature_file)],
        )
        .expect("couldn't prepare cpio archive");

        qemu_out_args.append(&mut vec![
                "-netdev".to_string(),
                "bridge,id=en1,br=controlnet".to_string(),
                "-device".to_string(),
                "virtio-net-pci,netdev=en1".to_string(),
                // "-chardev".to_string(),
                // "stdio,id=char0,mux=on,logfile=/tmp/foobar,signal=off".to_string(),
                // "-serial".to_string(),
                // "chardev:char0".to_string(),
                // "-mon".to_string(),
                // "chardev=char0".to_string(),
        ]);

        let mut click_vm = vm::start_click(
            FileSystem::CpioArchive(&cpio.path.to_string_lossy()),
            &qemu_out_args,
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
    let now = Instant::now();
    trigger_reconfiguration(config.bpfilter_program, config.signature_file)
        .expect("couldn't trigger reconfiguration");
    wait_until_reconfiguration_start(lines);

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
    let written = socket
        .send_to(&data, CONTROL_ADDR)
        .context("couldn't send packet")?;
    assert_eq!(written, data.len());

    Ok(())
}

fn wait_until_reconfiguration_start(lines: &mut Lines<BufReader<ChildStdout>>) {
    lines
        .filter_map(Result::ok)
        .find(|line| line.contains("init ebpf vm"));
}

fn wait_until_reconfiguration_end(lines: &mut Lines<BufReader<ChildStdout>>) {
    lines
        .filter_map(Result::ok)
        .find(|line| line.contains("init ebpf done"));
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
