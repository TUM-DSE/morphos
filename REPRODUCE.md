# Reproducing the MorphOS Evaluation



## Project Structure

The project is structured as follows:

```
.
├── benchmark: Contains the benchmarks for the system
├── ebpf: Contains the eBPF programs
├── examples: Contains system examples running the framework with different eBPF programs and Click configurations
├── helper: Contains a helper binary to, e.g., send reconfiguration packets to the Unikernel
├── libs: Contains the ubpf library, and the Click library containing the BPF elements
└── verifier: Contains the external verifier for the eBPF programs
```


## Measurement hosts

Git-clone this repository on both hotsts (adelaide, wilfred) **into the same absolute path** with the `--recursive` flag to also clone the submodules.

```bash
git clone https://github.com/TUM-DSE/morphos.git --recursive /scratch/$USER/morphos
```


## Setup: Adelaide, Device under Test

2. Run `nix develop` to enter the development environment with all system dependencies (the `nix` package manager is installed on the server)
3. Run `just build-dependencies` to build benchmarking runtime dependencies
5. Run `cargo install bpf-linker --version 0.9.14` to build `~/.cargo/bin/bpf-linker` required to compile ebpf programs
6. Run `just vm-image-init` to build Linux VM images
7. Run `make -C verifier build -j` to build the verifier


## Setup: Wilfred, Load Generator

2. Run `nix develop` to enter the development environment with all system dependencies (the `nix` package manager is installed on the server)
4. Run `just build-dependencies` to build benchmarking runtime dependencies


## Build

1. Run `just build-morphos` to build unikraft variants (calls `nix/unikraft.nix`)
2. Run `just TODO` to build and verify eBPF programs


## Running the Evaluation

Overview:

- `measure_firewall.py`: TODO hours for Fig. 13
- `measure_latency.py`: TODO hours for Fig. 1, 12
- `measure_reconfiguration.py`: TODO hours for Fig. 2, 7, 9
- `measure_throughput.py`: TODO hours for Fig. 1, 10, 11
- `just imagesize`: TODO hours for Fig. 8
- `just TODO-verification-etc`: TODO hours for Fig. 9


### Python benchmarks

After completing all of the above, you can launch the benchmarks on wilfred, the load generator.
The scripts will start VMs on adelaide, the device under test, and send test traffic from wilfred to adelaide.

```
python3 benchmark/pysrc/measure.py -c benchmark/conf/uk_adelaide_wilfred.cfg -vvv -o ./output
```

Unless other flags are specified, a measurement is skipped when it's output `*.log` file already exists.
Use other measurement scripts to run only a subset of tests:




## TODO Legacy Benchmarks

To run the benchmarks, you can run the following command inside the `benchmarks` directory:

```bash
make setup
cargo bench
```

This will run the benchmarks and generate a report with the results.

`cargo bench` Criterion reports (`startup` and `live-reconfigure`):
The reports are stored in the `target/criterion` directory.
See the source files for what environment variables you can pass.

Ordinary `cargo bench` reports (`throughput`, `memory`, and `latency`) are stored directly in the `target` directory.
You may limit the tests to be executed, e.g., with `cargo bench --features print-output --bench throughput -- --only="round-robin (BPFClassifier - JIT)"`.

Some benchmarks are implemented as ordinary rust binaries: `cargo run --bin bench-helper --features print-output`.


## TODO Misc Benchmarks

```
just imagesizes
just TODO-verification-etc
```

## Debugging

See also [README.md](README.md) for instructions on how to create unikraft debug builds.
