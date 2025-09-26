# ebpf programs

## Prerequisites

1. Install bpf-linker: `cargo install bpf-linker`
2. Import binary to path: `export PATH="/home/$USER/.cargo/bin:$PATH"`

## Build eBPF

```bash
make all
```

## Debug eBPF verification

Generate debug symbols for `nat` program. This also yields more useful verifier output.

```
RUSTFLAGS="-C debuginfo=2 -C link-arg=--btf" cargo build --bin nat --target bpfel-unknown-none -Z build-std=core
llvm-objdump -S ./target/bpfel-unknown-none/debug/nat > ./src/bin/nat.asm
../verifier/build/ubpf_verifier -f ./target/bpfel-unknown-none/debug/nat -k ../verifier/keys/ec_private_key.pem -o /tmp/foo -v 3
```


## Programs

| Program Name            | Program Type  | Description                                                  | Passes Verification |
|-------------------------|---------------|--------------------------------------------------------------|---------------------|
| dns-filter              | BPFFilter     | Drops DNS queries with `lmu.de`                              |                     |
| drop                    | BPFFilter     | Drops all packets                                            | ✅                   |
| ether-mirror            | BPFRewriter   | Mirrors ethernet destination & source addresses              | ✅                   |
| pass                    | BPFFilter     | Allows all packets                                           | ✅                   |
| rate-limiter            | BPFFilter     | Rate-limits incoming packets                                 | ✅                   |
| strip-ether-vlan-header | BPFRewriter   | Removes the Ethernet header                                  | ✅                   |
| target-port             | BPFFilter     | Drops all IPv4 packets with target port `12345`              | ✅                   |
| udp-tcp-classifier      | BPFClassifier | Classifies packets based on whether they're UDP, TCP or else | ✅                   |
| ...                     | ...           |                                                              |                      |
