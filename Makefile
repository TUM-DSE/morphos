build:
	rm -rf .unikraft/build/libclick/origin/click-a5384835a6cac10f8d44da4eeea8eaa8f8e6a0c2/elements/unikraft || true
	mkdir -p .unikraft/build/libclick/origin/click-a5384835a6cac10f8d44da4eeea8eaa8f8e6a0c2/elements/unikraft || true
	cp -r libs/click/unikraft .unikraft/build/libclick/origin/click-a5384835a6cac10f8d44da4eeea8eaa8f8e6a0c2/elements
	kraft build --no-configure --no-fetch --no-update --no-rootfs --sources-dir /scratch/paulz/.unikraft/sources --manifests-dir /scratch/paulz/.unikraft/manifests --log-type basic

run:
	./run.sh

bpf: dns-filter drop ether-mirror pass rate-limiter strip-ether-vlan-header target-port udp-tcp-classifier

dns-filter:
	cd ebpf && cargo xtask build-dns-filter-ebpf --release && (cp target/bpfel-unknown-none/release/dns-filter ../rootfs/dns-filter || true)

drop:
	cd ebpf && cargo xtask build-drop-ebpf --release && (cp target/bpfel-unknown-none/release/drop ../rootfs/drop || true)

ether-mirror:
	cd ebpf && cargo xtask build-ether-mirror-ebpf --release && (cp target/bpfel-unknown-none/release/ether-mirror ../rootfs/ether-mirror || true)

pass:
	cd ebpf && cargo xtask build-pass-ebpf --release && (cp target/bpfel-unknown-none/release/pass ../rootfs/pass || true)

rate-limiter:
	cd ebpf && cargo xtask build-rate-limiter-ebpf --release && (cp target/bpfel-unknown-none/release/rate-limiter ../rootfs/rate-limiter || true)

strip-ether-vlan-header:
	cd ebpf && cargo xtask build-strip-ether-vlan-header-ebpf --release && (cp target/bpfel-unknown-none/release/strip-ether-vlan-header ../rootfs/strip-ether-vlan-header || true)

target-port:
	cd ebpf && cargo xtask build-target-port-ebpf --release && (cp target/bpfel-unknown-none/release/target-port ../rootfs/target-port || true)

udp-tcp-classifier:
	cd ebpf && cargo xtask build-udp-tcp-classifier-ebpf --release && (cp target/bpfel-unknown-none/release/udp-tcp-classifier ../rootfs/udp-tcp-classifier || true)

disassemble-bpf:
	llvm-objdump -d @(BPF)

disassemble-jit-dump:
	objdump -D -b binary -mi386 -Maddr16,data16 rootfs/jit_dump.bin -Mintel

benchmark:
	cd benchmarks && cargo bench