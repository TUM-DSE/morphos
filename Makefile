build:
	# Hack to fix missing invalidation of copied Click elements
	rm -rf .unikraft/build/libclick/origin/click-a5384835a6cac10f8d44da4eeea8eaa8f8e6a0c2/elements/unikraft || true
	mkdir -p .unikraft/build/libclick/origin/click-a5384835a6cac10f8d44da4eeea8eaa8f8e6a0c2/elements/unikraft || true
	cp -r libs/click/unikraft .unikraft/build/libclick/origin/click-a5384835a6cac10f8d44da4eeea8eaa8f8e6a0c2/elements

	kraft build --log-type basic
downloadLibs:
	@nix develop .#unikraft --command bash -c 'sourceRoot=$$(pwd); eval "$$postUnpack"'

kill:
		sudo pkill -f "clicknet"
		sudo pkill -f "controlnet"

throughput.cpio: throughput.click
	rm -r /tmp/ukcpio-$(USER) || true
	mkdir -p /tmp/ukcpio-$(USER)
	cp ./throughput.click /tmp/ukcpio-$(USER)/config.click
	./libs/unikraft/support/scripts/mkcpio ./throughput.cpio /tmp/ukcpio-$(USER)

vm: throughput.cpio
	sudo taskset -c 3,4 qemu-system-x86_64 \
		-accel kvm -cpu max \
		-m 1024M -object memory-backend-file,id=mem,size=1024M,mem-path=/dev/hugepages,share=on \
    -mem-prealloc -numa node,memdev=mem \
		-netdev bridge,id=en0,br=clicknet \
		-device virtio-net-pci,netdev=en0 \
		-append " vfs.fstab=[\"initrd0:/:extract::ramfs=1:\"] --" \
		-kernel ./.unikraft/build/click_qemu-x86_64 \
		-initrd ./throughput.cpio \
		-nographic

vm-vhost: throughput.cpio
	sudo taskset -c 3,4 qemu-system-x86_64 \
		-accel kvm -cpu max \
		-m 1024M -object memory-backend-file,id=mem,size=1024M,mem-path=/dev/hugepages,share=on \
    -mem-prealloc -numa node,memdev=mem \
    -chardev socket,id=char1,path=/tmp/vhost-user0,server \
    -netdev type=vhost-user,id=hostnet1,chardev=char1  \
    -device virtio-net-pci,netdev=hostnet1,id=net1,mac=52:54:00:00:00:14 \
		-append " vfs.fstab=[\"initrd0:/:extract::ramfs=1:\"] --" \
		-kernel ./.unikraft/build/click_qemu-x86_64 \
		-initrd ./throughput.cpio \
		-nographic


vhost-user:
	sudo ./result-examples/bin/dpdk-vhost -l 5-8 -n 4 --socket-mem 1024 -- --socket-file /tmp/vhost-user0 --client -p 1 --stats 1


vpp-notes:
# sudo vpp -c ./vpp.conf
	sudo vppctl -s /tmp/vpp-cli
# show log
# show interface
	create vhost-user socket /tmp/vhost-user0
	set int state VirtualEthernet0/0/0 up
	set interface l2 xconnect GigabitEthernet0/8/0.300 GigabitEthernet0/9/0.300


perf-kvm-top:
	sudo perf kvm --guestkallsyms=./.unikraft/build/click_qemu-x86_64.sym --guestvmlinux=.unikraft/build/click_qemu-x86_64.dbg top -p $(pgrep qemu)

perf-kvm-record:
	sudo perf kvm --guestkallsyms=./.unikraft/build/click_qemu-x86_64.sym --guestvmlinux=.unikraft/build/click_qemu-x86_64.dbg record -g -p $(pgrep qemu)
	sudo perf kvm --guestkallsyms=./.unikraft/build/click_qemu-x86_64.sym --guestvmlinux=.unikraft/build/click_qemu-x86_64.dbg report

perf-qemu-record:
	sudo perf record -g -p $(pgrep qemu)
	sudo perf script > perf.trace
