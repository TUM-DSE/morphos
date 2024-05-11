# Measurements

## Startup Time

Measured from the time of startup of our benchmarking harness to the time of the first
packet being received by the device, indicated by the device printing a message to stdout.

### Baseline

Click Configuration:
```
FromDevice
  -> Print('Received packet from device')
  -> IPPrint
  -> ToDevice;
```

Results:
```
Benchmark 1: ./target/release/click-benchmark
  Time (mean ± σ):     462.6 ms ±  57.7 ms    [User: 2.4 ms, System: 10.8 ms]
  Range (min … max):   439.1 ms … 776.4 ms    100 runs
```