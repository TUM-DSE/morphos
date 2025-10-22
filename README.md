# Network Function Virtualization with MorphOS

This repository contains the source code and implementation of a framework that integrates eBPF into Unikernel-based Virtual Network Functions (VNFs).

The project leverages the Click Modular Router and eBPF to enhance packet processing flexibility, providing a secure and dynamic alternative to traditional VNFs. 
Key features include live reconfigurability without downtime and state retention, and decoupled eBPF verification for secure JIT compilation. 

We also provide a set of benchmarks to evaluate the performance, a set of examples showcasing the capabilities of the
system, and a set of eBPF programs to be used with the system.

This work is based on [app-click](https://github.com/unikraft/app-click).

Find instructions on how to reproduce our measurements in [REPRODUCE.md](REPRODUCE.md).

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

## Project Setup

1. Clone the repository with the `--recursive` flag to also clone the submodules
2. Install the `nix` package manager
3. Run `nix develop .#fhs` to enter the development environment with all system dependencies
4. Run `just downloadLibs` to download unikraft libraries as pinned by `flake.*`

### Building the Unikernel

To build the unikernel, run the following command inside the project directory:

```bash
make build
```

### Building the external verifier

To build the external verifier, run the following command inside the `verifier` directory:

```bash
make build
```

### Building the provided eBPF programs

To build the example eBPF programs, run the following command inside the `ebpf` directory:

```bash
make all
```

This will automatically build all the eBPF programs in the `ebpf` directory. If you want to directly verify the programs and generate signatures, you can add the `VERIFY` flag.

```bash
make all VERIFY=1
```

If you make any changes to the eBPF programs, you synchronize them with the vendored binaries inside the benchmarks and examples directories by running:

```bash
make sync
```

Further information about our ebpf build system: https://aya-rs.dev/book/start/development/

## Running the Benchmarks

After completing all of the above, you can run the benchmarks:

```
# localhost doesn't support Interface.VPP (vhost-user):
python3 benchmark/pysrc/measure_throughput.py -c benchmark/conf/uk_localhost.cfg -vvv

# multihost:
python3 benchmark/pysrc/measure_throughput.py -c benchmark/conf/uk_adelaide_wilfred.cfg -vvv
```

See `-h` for more benchmarking options. When run on other hosts than `adelaide`, the config file and `benchmark/conf/ssh_config_doctor_cluster` may need adjusting.

## Running the Legacy Benchmarks

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


## Example use cases

The `examples` folder holds multiple full examples with which the framework can be run. All examples except for `chain` and `showcase` can be setup using `make setup` and started using `make run`.

* `chain` "chains" multiple VMs with different use cases to each other, as explained in Section 6.2.1 (Extensibility & Customizability).
   It can be started by using `make setup` for creating the network devices, and `make up` to start the VMs.
* `dns-filter` blocks all DNS resolutions against a specific domain.
* `drop` drops all packets.
* `ether-mirror` mirrors the raw Ethernet packets back to the sender.
* `pass` allows all packets.
* `rate-limiter` applies token-based rate limiting on a per-IP basis.
* `showcase` contains a TUI which allows live reconfiguration, triggering reconfiguration, sending packets, and visualizing all received and blocked packets.
    It can be setup using `make setup` and started by using `make tui`. 
* `state-migration` contains an example which allows testing a state migration, as explained in Section 6.3 (State Migration). The state migration can be triggered by using the helper tools.
* `strip-ether-vlan-header` removes the Ethernet header from incoming packets.
* `target-port` blocks all packets to a specific port.
* `udp-tcp-classifier` classifies packets, depending on whether they're TCP or UDP packets.

## Helpers

The `helper` subdirectory contains helpers for the framework:
* `cargo run -- reconfigure [PROGRAM] [SIGNATURE]`: Sends a control packet to the VM and triggers reconfiguration for the BPF Element with ID 1
* `cargo run -- send-packet`: Sends a UDP packet to the VM
* `cargo run -- send-tcp-packet`: Sends a TCP packet to the VM

## Verifier

The `verifier` subdirectory contains the external PREVAIL-based verifier. After building it, it can be invoked using

```bash
build/ubpf_verifier -f [PROGRAM] -k keys/ec_private_key.pem -o [SIGNATURE_OUTPUT]
```

**Keep in mind that the supplied private keys are for test purposes only. They are deliberately shared to make it easy to test.**
