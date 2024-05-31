sudo qemu-system-x86_64 \
  -accel kvm \
  -cpu max \
  -netdev bridge,id=en0,br=clicknet -device virtio-net-pci,netdev=en0 \
  -netdev bridge,id=en1,br=controlnet -device virtio-net-pci,netdev=en1 \
  -append "netdev.ip=172.44.0.2/24:172.44.0.1 vfs.fstab=[initrd0:/:extract::ramfs=1:] --" \
  -kernel ../.unikraft/build/click_qemu-x86_64 \
  -initrd configurations/out/pass.cpio \
  -nographic
