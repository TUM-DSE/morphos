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

## Initial Setup

1. Clone the repository with the `--recursive` flag to also clone the submodules
2. Run `nix develop` to enter the development environment with all system dependencies (the `nix` package manager is installed on the server)
3. Run `just downloadLibs` to download unikraft libraries as pinned by `flake.*`
4. Run `just build-dependencies` to build benchmarking runtime dependencies
5. Run `cargo install bpf-linker --version 0.9.14` to build `~/.cargo/bin/bpf-linker` required to compile ebpf programs
