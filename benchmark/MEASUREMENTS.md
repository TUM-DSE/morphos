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
  Time (mean ± σ):     466.6 ms ±  72.0 ms    [User: 2.4 ms, System: 10.8 ms]
  Range (min … max):   433.5 ms … 859.5 ms    100 runs
```