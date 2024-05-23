#!/bin/sh

.unikraft/unikraft/support/scripts/mkcpio .unikraft/build/initramfs-x86_64.cpio rootfs

sudo ip link set dev clicknet down 2> /dev/null
sudo ip link del dev clicknet 2> /dev/null
sudo ip link add dev clicknet type bridge
sudo ip address add 172.44.0.1/24 dev clicknet
sudo ip link set dev clicknet up

sudo ip link set dev controlnet down 2> /dev/null
sudo ip link del dev controlnet 2> /dev/null
sudo ip link add dev controlnet type bridge
sudo ip address add 173.44.0.1/24 dev controlnet
sudo ip link set dev controlnet up

sudo qemu-system-x86_64 \
  -accel kvm \
  -cpu max \
  -netdev bridge,id=en0,br=clicknet -device virtio-net-pci,netdev=en0 \
  -netdev bridge,id=en1,br=controlnet -device virtio-net-pci,netdev=en1 \
  -append "vfs.fstab=[\"initrd0:/:extract::ramfs=1:\"] --" \
  -kernel .unikraft/build/click_qemu-x86_64 \
  -initrd .unikraft/build/initramfs-x86_64.cpio \
  -nographic
