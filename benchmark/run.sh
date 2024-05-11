#!/bin/sh

hyperfine --warmup 3 --runs 100 ./target/release/click-benchmark

# This is a workaround for the terminal not echoing characters after the program exits
stty echo