#!/bin/sh

sudo ip netns delete clickns
sudo ip netns add clickns

sudo ip netns exec clickns ip link add dev virbr0 type bridge
sudo ip netns exec clickns ip address add 172.44.0.1/24 dev virbr0
sudo ip netns exec clickns ip link set dev virbr0 up

sudo ip netns exec clickns kraft run --network=bridge:virbr0