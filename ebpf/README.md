# ebpf programs

## Prerequisites

1. Install bpf-linker: `cargo install bpf-linker`

## Build eBPF

```bash
make all
```

## Programs

| Program Name            | Program Type  | Description                                                  | Passes Verification |
|-------------------------|---------------|--------------------------------------------------------------|---------------------|
| dns-filter              | BPFFilter     | Drops DNS queries with `lmu.de`                              |                     |
| drop                    | BPFFilter     | Drops all packets                                            | ✅                   |
| ether-mirror            | BPFRewriter   | Mirrors ethernet destination & source addresses              | ✅                   |
| pass                    | BPFFilter     | Allows all packets                                           | ✅                   |
| rate-limiter            | BPFFilter     | Rate-limits incoming packets                                 |                     |
| strip-ether-vlan-header | BPFRewriter   | Removes the Ethernet header                                  |                     |
| target-port             | BPFFilter     | Drops all IPv4 packets with target port `12345`              | ✅                   |
| udp-tcp-classifier      | BPFClassifier | Classifies packets based on whether they're UDP, TCP or else | ✅                   |
