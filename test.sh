#!/bin/sh

sudo ip netns delete clickns
sudo ip netns add clickns

sudo ip netns exec clickns ip link add dev virbr0 type bridge
sudo ip netns exec clickns ip address add 172.44.0.1/24 dev virbr0
sudo ip netns exec clickns ip link set dev virbr0 up

sudo ip netns exec clickns qemu-system-x86_64 \
    -netdev bridge,id=en0,br=virbr0 -device virtio-net-pci,netdev=en0 \
    -append "netdev.ipv4_addr=172.44.0.2 netdev.ipv4_gw_addr=172.44.0.1 netdev.ipv4_subnet_mask=255.255.255.0 --" \
    -kernel build/click_qemu-x86_64 \
    -initrd elements/forward.click \
    -nographic

sudo ip netns exec clickns qemu-system-x86_64 \
    -netdev bridge,id=en1,br=virbr0 -device virtio-net-pci,netdev=en1 \
    -append "netdev.ipv4_addr=172.44.0.3 netdev.ipv4_gw_addr=172.44.0.1 netdev.ipv4_subnet_mask=255.255.255.0 --" \
    -kernel build/click_qemu-x86_64 \
    -initrd elements/discard.click \
    -nographic
