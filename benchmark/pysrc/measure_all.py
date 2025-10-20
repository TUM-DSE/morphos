from measure import Measurement
import measure_throughput
import measure_firewall
import measure_latency
import measure_misc
import measure_reconfiguration
from logging import (info, debug, error, warning,
                     DEBUG, INFO, WARN, ERROR)

def main():
    measurement = Measurement()

    # estimate runtimes
    info("")
    measure_throughput.main(measurement, plan_only=True)
    measure_firewall.main(measurement, plan_only=True)
    measure_latency.main(measurement, plan_only=True)
    measure_reconfiguration.main(measurement, plan_only=True)
    measure_misc.main(measurement, plan_only=True)

    info("Running benchmarks ...")
    info("")
    measure_throughput.main(measurement)
    measure_firewall.main(measurement)
    measure_latency.main(measurement)
    measure_reconfiguration.main(measurement)
    measure_misc.main(measurement)

if __name__ == "__main__":
    main()
