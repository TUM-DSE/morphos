build:
	# Hack to fix missing invalidation of copied Click elements
	rm -rf .unikraft/build/libclick/origin/click-a5384835a6cac10f8d44da4eeea8eaa8f8e6a0c2/elements/unikraft || true
	mkdir -p .unikraft/build/libclick/origin/click-a5384835a6cac10f8d44da4eeea8eaa8f8e6a0c2/elements/unikraft || true
	cp -r libs/click/unikraft .unikraft/build/libclick/origin/click-a5384835a6cac10f8d44da4eeea8eaa8f8e6a0c2/elements

	kraft build --log-type basic

kill:
		sudo pkill -f "clicknet"
		sudo pkill -f "controlnet"

throughput.cpio: throughput.click
	mkdir /tmp/ukcpio-$(USER)
	cp ./throughput.click /tmp/ukcpio-$(USER)
	.unikraft/unikraft/support/scripts/mkcpio ./throughput.cpio /tmp/ukcpio-$(USER)

vm: throughput.cpio
	sudo taskset -c 3,4 qemu-system-x86_64 -accel kvm -cpu max -netdev bridge,id=en0,br=clicknet -device virtio-net-pci,netdev=en0 -append " vfs.fstab=[\"initrd0:/:extract::ramfs=1:\"] --" -kernel ./.unikraft/build/click_qemu-x86_64 -initrd ./throughput.cpio -nographic

perf-kvm-top:
	sudo perf kvm --guestkallsyms=./.unikraft/build/click_qemu-x86_64.sym --guestvmlinux=.unikraft/build/click_qemu-x86_64.dbg top -p $(pgrep qemu)

perf-kvm-record:
	sudo perf kvm --guestkallsyms=./.unikraft/build/click_qemu-x86_64.sym --guestvmlinux=.unikraft/build/click_qemu-x86_64.dbg record -g -p $(pgrep qemu)
	sudo perf kvm --guestkallsyms=./.unikraft/build/click_qemu-x86_64.sym --guestvmlinux=.unikraft/build/click_qemu-x86_64.dbg report

perf-qemu-record:
	sudo perf record -g -p $(pgrep qemu)
	sudo perf script > perf.trace
