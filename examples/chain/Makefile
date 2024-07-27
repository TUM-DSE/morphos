up:
	tmux new-session -d -s click_examples_chain

	# Start firewall
	tmux new-window -t click_examples_chain:1 -n firewall 'sudo qemu-system-x86_64 \
	  -accel kvm \
	  -cpu max \
	  -netdev bridge,id=en0,br=clicknet -device virtio-net-pci,netdev=en0,mac=00:10:FA:B3:49:39 \
	  -netdev bridge,id=en1,br=controlnet -device virtio-net-pci,netdev=en1,mac=00:10:FA:47:DF:2C \
	  -fsdev local,security_model=passthrough,id=hvirtio1,path=firewall -device virtio-9p-pci,fsdev=hvirtio1,mount_tag=fs1 \
	  -append "vfs.fstab=[\"fs1:/:9pfs\"] --" \
	  -kernel ../../.unikraft/build/click_qemu-x86_64 \
	  -nographic'

	tmux new-window -t click_examples_chain:2 -n load-balancer 'sudo qemu-system-x86_64 \
	  -accel kvm \
	  -cpu max \
	  -netdev bridge,id=en0,br=clicknet -device virtio-net-pci,netdev=en0,mac=00:10:FA:A2:7D:34 \
	  -netdev bridge,id=en1,br=controlnet -device virtio-net-pci,netdev=en1,mac=00:10:FA:CC:FD:59 \
	  -fsdev local,security_model=passthrough,id=hvirtio1,path=load-balancer -device virtio-9p-pci,fsdev=hvirtio1,mount_tag=fs1 \
	  -append "vfs.fstab=[\"fs1:/:9pfs\"] --" \
	  -kernel ../../.unikraft/build/click_qemu-x86_64 \
	  -nographic'

	tmux new-window -t click_examples_chain:3 -n app-1 'sudo qemu-system-x86_64 \
	  -accel kvm \
	  -cpu max \
	  -netdev bridge,id=en0,br=clicknet -device virtio-net-pci,netdev=en0,mac=00:10:FA:D7:BC:C4 \
	  -fsdev local,security_model=passthrough,id=hvirtio1,path=app-1 -device virtio-9p-pci,fsdev=hvirtio1,mount_tag=fs1 \
	  -append "vfs.fstab=[\"fs1:/:9pfs\"] --" \
	  -kernel ../../.unikraft/build/click_qemu-x86_64 \
	  -nographic'

	tmux new-window -t click_examples_chain:4 -n app-2 'sudo qemu-system-x86_64 \
	  -accel kvm \
	  -cpu max \
	  -netdev bridge,id=en0,br=clicknet -device virtio-net-pci,netdev=en0,mac=00:10:FA:39:EA:2C \
	  -fsdev local,security_model=passthrough,id=hvirtio1,path=app-2 -device virtio-9p-pci,fsdev=hvirtio1,mount_tag=fs1 \
	  -append "vfs.fstab=[\"fs1:/:9pfs\"] --" \
	  -kernel ../../.unikraft/build/click_qemu-x86_64 \
	  -nographic'

	tmux attach-session -t click_examples_chain

down:
	tmux kill-session -t click_examples_chain

setup:
	sudo kraft net rm clicknet || true
	sudo kraft net create clicknet -n 172.44.0.1/24

	sudo kraft net rm controlnet || true
	sudo kraft net create controlnet -n 173.44.0.1/24

	sudo kraft net rm webserver || true
	sudo kraft net create webserver -n 174.44.0.1/24