#!/bin/sh

mkdir results || true
hyperfine --warmup 3 --runs 100 --export-json results/click-benchmark.json \
  './target/release/click-benchmark configurations/minimal.click' \
  './target/release/click-benchmark configurations/switch-2ports.click -netdev bridge,id=en1,br=clicknet -device virtio-net-pci,netdev=en1,id=en1' \
  './target/release/click-benchmark configurations/router.click -netdev bridge,id=en1,br=clicknet -device virtio-net-pci,netdev=en1,id=en1 -netdev bridge,id=en2,br=clicknet -device virtio-net-pci,netdev=en2,id=en2 -netdev bridge,id=en3,br=clicknet -device virtio-net-pci,netdev=en3,id=en3' \
  './target/release/click-benchmark configurations/print-pings.click' \
  './target/release/click-benchmark configurations/thomer-nat.click'

# This is a workaround for the terminal not echoing characters after the program exits
stty echo

# Visualize the results
python3 scripts/plot_histogram.py results/click-benchmark.json -o results/hist.png --title 'Startup Time' --labels minimal,switch-2ports,router,print-pings,thomer-nat
python3 scripts/plot_whisker.py results/click-benchmark.json -o results/box.png --sort-by median --title 'Startup Time' --labels minimal,switch-2ports,router,print-pings,thomer-nat