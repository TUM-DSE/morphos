// Module responsible for handling *startup* benchmarks.
//
// # Benchmarking procedure
// 1. Start qemu with the appropriate arguments
// 2. Exit the program once "Received packet" was printed to qemu's stdout
// 3. In a separate thread, continuously send a lot of UDP packets to qemu (in order to trigger the "Received packet" message).

use click_benchmark::startup_base::{self, Configuration};
use click_benchmark::vm::{self, wait_until_ready, FileSystem, DATA_ADDR};
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, SamplingMode};
use std::thread;
use std::time::{Duration, Instant};

const CONFIGURATIONS: &[Configuration] = &[
    Configuration {
        name: "minimal",
        click_configuration: "configurations/minimal.click",
        vm_extra_args: &[],
    },
    Configuration {
        name: "print-pings",
        click_configuration: "configurations/print-pings.click",
        vm_extra_args: &[],
    },
    Configuration {
        name: "switch-2ports",
        click_configuration: "configurations/switch-2ports.click",
        vm_extra_args: &[
            "-netdev",
            "bridge,id=en1,br=clicknet",
            "-device",
            "virtio-net-pci,netdev=en1,id=en1",
        ],
    },
    Configuration {
        name: "thomer-nat",
        click_configuration: "configurations/thomer-nat.click",
        vm_extra_args: &[],
    },
    Configuration {
        name: "router",
        click_configuration: "configurations/router.click",
        vm_extra_args: &[
            "-netdev",
            "bridge,id=en1,br=clicknet",
            "-device",
            "virtio-net-pci,netdev=en1,id=en1",
            "-netdev",
            "bridge,id=en2,br=clicknet",
            "-device",
            "virtio-net-pci,netdev=en2,id=en2",
            "-netdev",
            "bridge,id=en3,br=clicknet",
            "-device",
            "virtio-net-pci,netdev=en3,id=en3",
        ],
    },
];

pub fn startup(c: &mut Criterion) {
    let mut group = c.benchmark_group("startup");

    group.sample_size(10);
    group.measurement_time(Duration::from_secs(120));
    group.sampling_mode(SamplingMode::Flat);

    thread::spawn(|| {
        startup_base::send_packet_loop().expect("error in send packet loop");
    });

    for config in CONFIGURATIONS {
        group.bench_with_input(
            BenchmarkId::from_parameter(config.name),
            config,
            |b, config| {
                b.iter_custom(|iters| {
                    let mut sum = Duration::new(0, 0);
                    for _ in 0..iters {
                        sum += startup_base::run_benchmark(config);
                    }
                    sum
                });
            },
        );
    }

    group.finish();
}

criterion_group!(benches, startup);
criterion_main!(benches);
