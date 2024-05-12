# Measurements

## Startup Time

Measured from the time of startup of our benchmarking harness to the time of the first
packet being received by the device, indicated by the device printing a message to stdout.

### Baseline

Click Configuration:
```
FromDevice
  -> Print('Received packet from device')
  -> Discard;
```

Results:
```
Benchmark 1: ./target/release/click-benchmark
  Time (mean ± σ):     355.6 ms ±  88.1 ms    [User: 0.9 ms, System: 1.8 ms]
  Range (min … max):   299.7 ms … 703.4 ms    100 runs
```