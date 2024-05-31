#!/bin/sh

mkdir results || true

../.unikraft/unikraft/support/scripts/mkcpio configurations/out/pass.cpio configurations/pass
./target/release/click-benchmark reconfigure configurations/out/pass.cpio pass

# This is a workaround for the terminal not echoing characters after the program exits
stty echo