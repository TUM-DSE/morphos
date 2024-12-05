import autotest as autotest
from configparser import ConfigParser
from argparse import (ArgumentParser, ArgumentDefaultsHelpFormatter, Namespace,
                      FileType, ArgumentTypeError)
from argcomplete import autocomplete
from logging import (info, debug, error, warning,
                     DEBUG, INFO, WARN, ERROR)
from server import Host, Guest, LoadGen
from loadlatency import LoadLatencyTest
from enums import Machine, Interface, Reflector
from measure import Measurement
from util import safe_cast
from typing import Iterator, cast
import time
from conf import G


def main(measurement: Measurement, plan_only: bool = False) -> None:
    # general measure init
    host, loadgen = measurement.hosts()

    info("VNF execution plan:")
    info("Incremental tests not supported. Running tests for maybe 5 min.")

    if plan_only:
        return

    # loadgen: set up interfaces and networking

    debug('Binding loadgen interface')
    try:
        loadgen.delete_nic_ip_addresses(loadgen.test_iface)
    except Exception:
        pass
    loadgen.bind_test_iface()
    loadgen.setup_hugetlbfs()


    interface = Interface.VMUX_PT
    # TODO put early skip check here
    with measurement.virtual_machine(interface) as guest:

        # guest: set up interfaces and networking

        debug("Detecting guest test interface")
        guest.detect_test_iface()
        guest.bind_test_iface()
        guest.setup_hugetlbfs()

        # the actual test

        loadgen.stop_moongen_reflector()
        loadgen.start_moongen_reflector()

        test = LoadLatencyTest(
            machine=Machine.PCVM,
            interface=interface,
            mac=loadgen.test_iface_mac,
            qemu="measure-vnf", # abuse this as comment field
            vhost=False,
            ioregionfd=False,
            reflector=Reflector.MOONGEN,
            rate=1,
            size=64,
            runtime=30,
            repetitions=1,
            warmup=False,
            cooldown=False,
            outputdir=G.OUT_DIR,
        )
        guest_: LoadGen = cast(LoadGen, guest) # trust me bro, this works
        test.run(guest_)

        loadgen.stop_moongen_reflector()

    interface = Interface.VFIO

    with measurement.virtual_machine(interface) as guest:
        # guest: set up interfaces and networking

        debug("Detecting guest test interface")
        guest.exec("sudo modprobe ice")
        guest.bind_device(guest.test_iface_addr, "ice")
        guest.exec(f"sudo ip link set {guest.test_iface} up")

        # the actual test

        loadgen.stop_moongen_reflector()
        loadgen.start_moongen_reflector()

        # TODO check if we can skip
        time.sleep(1) # give ip link time to come up
        info("Running kni-latency")
        remote_output_file = "/tmp/out.log"
        guest.exec(
                f"{measurement.config['guest']['moonprogs_dir']}/../kni-latency/kni-latency "
                f"{loadgen.test_iface_mac} {guest.test_iface} "
                f"| tee {remote_output_file}"
                )
        guest.copy_from(remote_output_file,
                    f"{G.OUT_DIR}/measure_vnf_rep0.log")

        loadgen.stop_moongen_reflector()


if __name__ == "__main__":
    measurement = Measurement()
    main(measurement)
