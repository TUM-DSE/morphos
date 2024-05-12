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
  Time (mean ± σ):     404.9 ms ±  88.1 ms    [User: 0.8 ms, System: 3.3 ms]
  Range (min … max):   364.8 ms … 933.4 ms    100 runs
```