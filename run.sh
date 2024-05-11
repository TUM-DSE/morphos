#!/bin/sh

sudo qemu-system-x86_64 \
  -netdev bridge,id=en0,br=clicknet -device virtio-net-pci,netdev=en0 \
  -append "netdev.ipv4_addr=172.44.0.2 netdev.ipv4_gw_addr=172.44.0.1 netdev.ipv4_subnet_mask=255.255.255.0 --" \
  -kernel .unikraft/build/click_qemu-x86_64 \
  -initrd .unikraft/build/initramfs-x86_64.cpio \
  -nographic
