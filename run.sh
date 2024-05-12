#!/bin/sh

.unikraft/unikraft/support/scripts/mkcpio .unikraft/build/initramfs-x86_64.cpio rootfs

sudo qemu-system-x86_64 \
  -accel kvm \
  -netdev bridge,id=en0,br=clicknet -device virtio-net-pci,netdev=en0 \
  -append "netdev.ip=172.44.0.2/24:172.44.0.1 vfs.fstab=[\"initrd0:/:extract::ramfs=1:\"] --" \
  -kernel .unikraft/build/click_qemu-x86_64 \
  -initrd .unikraft/build/initramfs-x86_64.cpio \
  -nographic
