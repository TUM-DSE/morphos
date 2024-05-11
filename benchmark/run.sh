#!/bin/sh

mkdir results || true
hyperfine --warmup 3 --runs 100 ./target/release/click-benchmark --export-json results/click-benchmark.json

# This is a workaround for the terminal not echoing characters after the program exits
stty echo

# Visualize the results
python3 scripts/plot_histogram.py results/click-benchmark.json -o results/hist.png --labels 'Baseline: ClickOS Startup Time'