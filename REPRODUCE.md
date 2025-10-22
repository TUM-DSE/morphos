# Reproducing the MorphOS Evaluation



## Project Structure

The project is structured as follows:

```
.
├── benchmark: Contains the benchmarks for the system
├── ebpf: Contains the eBPF programs
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

Connect to adelaide, change into the `/scratch/$user/morphos` directory and prepare dependencies:

1. Run `nix develop` to enter the development environment with all system dependencies (the `nix` package manager is installed on the server)
2. Run `just build-dependencies` to build benchmarking runtime dependencies
3. Run `cargo install bpf-linker --version 0.9.14` to build `~/.cargo/bin/bpf-linker` required to compile ebpf programs
4. Run `just vm-image-init` to build Linux VM images
5. Run `just build-verifier` to build the verifier


## Setup: Christina, Load Generator

Next, on the second host, some dependencies are also needed.

1. Run `nix develop` to enter the development environment with all system dependencies (the `nix` package manager is installed on the server)
2. Run `just build-dependencies` to build benchmarking runtime dependencies


## Build

1. Run `just build-morphos` to build unikraft variants (calls `nix/unikraft.nix`)
2. Run `just build-ebpf` to build and verify eBPF programs


## Running the Evaluation

After completing all of the above, you can launch the benchmarks on christina, the load generator.

Overview:

- `measure_throughput.py`: 7.9 hours for Fig. 1, 10, 11
- `measure_firewall.py`: 3.3 hours for Fig. 13
- `measure_latency.py`: 0.5 hours for Fig. 1, 12
- `measure_reconfiguration.py`: 0.7 hours for Fig. 2, 7, 9
- `measure_misc.py`: 0.1 hours for Fig. 8, 9

Execute the python scripts on **christina**.
The scripts will start VMs on adelaide, the device under test, and send test traffic from christina to adelaide.


```
python3 benchmark/pysrc/measure_all.py -c benchmark/conf/uk_adelaide_wilfred.cfg -vvv -o ./output
```

Unless other flags are specified, a measurement is skipped when it's output `*.log` file already exists.
Use other measurement scripts to run only a subset of tests.


## Plotting

Finally, plot the results to pdfs.

```
make -C benchmark/plotting all -B DATA_DIR=./output TODO=./output-plots
```

Plot graphs individually, e.g., with:

```

make -C benchmark/plotting throughput.pdf -B DATA_DIR=./output TODO=./output-plots
```


## Debugging

See also [README.md](README.md) for instructions on how to create unikraft debug builds.
