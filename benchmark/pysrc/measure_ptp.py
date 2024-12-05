import autotest as autotest
from logging import (info, debug, error, warning, getLogger,
                     DEBUG, INFO, WARN, ERROR)
from server import Host, Guest, LoadGen, MultiHost
from enums import Machine, Interface, Reflector
from measure import Bench, AbstractBenchTest, Measurement, end_foreach
from util import safe_cast, product_dict
import time
from os.path import isfile, join as path_join
from root import *
from dataclasses import dataclass, field
from pandas import DataFrame
from conf import G

@dataclass
class PTPTest(AbstractBenchTest):

    interface: Interface # VMUX interface
    mode: str # can be: dpdk-guest, dpdk-host, ptp4l-guest, ptp4l-host

    def test_infix(self):
        interface = "none" if self.interface is None else self.interface.value
        return f"ptp_{self.mode}_{interface}"

    def per_vm_output_path(self, repetition: int, vm_nr: int) -> str:
        return f"{self.output_filepath(repetition)}.vm{vm_nr}"

    def estimated_runtime(self) -> float:
        """
        estimate time needed to run this benchmark excluding boot time in seconds
        """
        return DURATION_S * self.repetitions

    def find_error(self, repetition: int) -> bool:
        failure = False
        if not self.output_json:
            file = self.output_filepath(repetition)
            if os.stat(file).st_size == 0:
                error(f"Some ptp tests returned errors:\n{file}")
                failure = True
        return failure

    def summarize(self, repetition: int) -> DataFrame:
        print("summarize")
        return None

def strip_subnet_mask(ip_addr: str):
    return ip_addr[ : ip_addr.index("/")]


def main(measurement: Measurement, plan_only: bool = False) -> None:
    global DURATION_S

    host, loadgen = measurement.hosts()

    DURATION_S = 15*20 if not G.BRIEF else 80

    # set up test plan
    interfaces = [
          Interface.VFIO,
          Interface.VMUX_PT,
          Interface.VMUX_DPDK_E810,
          Interface.VMUX_MED ]

    modes = [
        "dpdk-guest",
        # "dpdk-host",
        # "ptp4l-host"
    ]
    num_vms = [ 1, 2 ]
    repetitions = 3

    if G.BRIEF:
        interfaces = [ Interface.VFIO ]
        # interfaces = [ Interface.VMUX_MED ]
        modes = [ "dpdk-guest" ]
        # num_vms = [ 1 ]
        num_vms = [ 2 ]
        repetitions = 3

    test_matrix = dict(
        repetitions=[ repetitions ],
        num_vms=num_vms,
        interface=interfaces, # [ interface.value for interface in interfaces],
        mode=modes
    )
    def exclude(test):
        return ((test.num_vms > 1) and ("host" in test.mode)) or \
                ((test.num_vms > 1) and (test.interface.is_passthrough()))
    tests = PTPTest.list_tests(test_matrix, exclude_test=exclude)
    if not G.BRIEF:
        tests += [ PTPTest(interface=None, mode="dpdk-host", num_vms=0, repetitions=repetitions) ]
    args_reboot = ["mode", "interface", "num_vms", "repetitions"]

    info(f"PTP Test execution plan:")
    PTPTest.estimate_time2(tests, args_reboot)

    with Bench(tests=tests, args_reboot = args_reboot, brief = G.BRIEF) as (bench, bench_tests):
        for [mode, interface, num_vms], test in bench.multi_iterator(bench_tests, ["mode", "interface", "num_vms"]):
            assert len(test) == 1 # we have looped through all variables now, right?
            test = test[0]
            # test = PTPTest(interface=iface, mode="dpdk-guest", num_vms=0, repetitions=1)
            for repetition in range(test.repetitions):
                run_test(test, repetition=repetition)
            bench.done(test)


def run_test(ptp_test: PTPTest, repetition=0):
    host, loadgen = measurement.hosts()

    info('Binding loadgen interface')
    loadgen.modprobe_test_iface_drivers()
    loadgen.release_test_iface() # bind linux driver

    try:
        loadgen.delete_nic_ip_addresses(loadgen.test_iface)
    except Exception:
        pass

    loadgen.setup_test_iface_ip_net()

    # Test ptpclient on host
    if ptp_test.mode == "dpdk-host":
        # cleanup
        try:
            host.kill_guest()
        except Exception as e:
            error("Could not kill guest VMs")
            error(e.with_traceback())
            pass
        loadgen.stop_ptp4l()
        host.stop_ptp_client()

        info("Testing DPDK PTP Client on host")
        host.bind_test_iface()
        host.modprobe_test_iface_drivers()

        info("Start ptp client")
        out_dir = ptp_test.output_filepath(repetition)

        info("Start ptp server")
        loadgen.start_ptp4l()

        remote_path = "/tmp/out.txt"
        host.start_ptp_client(remote_path)
        time.sleep(DURATION_S)

        host.stop_ptp_client()
        loadgen.stop_ptp4l()

        # remove unnecessary information
        host.exec(f"grep -oP \"Delta between master and slave.*?:\K-?\d+\" {remote_path} | tail -n +2 | sudo tee {remote_path}.filtered")

        host.copy_from(f"{remote_path}.filtered", f"{out_dir}.filtered")
        host.copy_from(f"{remote_path}", out_dir)

        return

    # boot VMs
    with measurement.virtual_machines(ptp_test.interface, num=ptp_test.num_vms) as guests:
        # cleanup
        loadgen.stop_ptp4l()
        def foreach_parallel(i, guest): # pyright: ignore[reportGeneralTypeIssues]
            guest.stop_ptp_client()
        end_foreach(guests, foreach_parallel)

        info("Testing DPDK PTP Client on host")
        def foreach_parallel(i, guest): # pyright: ignore[reportGeneralTypeIssues]
            guest.bind_test_iface()
            guest.modprobe_test_iface_drivers()
        end_foreach(guests, foreach_parallel)

        info("Start ptp client")
        # out_dir = ptp_test.output_filepath(repetition)

        info("Start ptp server")
        loadgen.start_ptp4l()

        remote_path = "/tmp/out.txt"
        def foreach_parallel(i, guest): # pyright: ignore[reportGeneralTypeIssues]
            guest.start_ptp_client(remote_path)
        end_foreach(guests, foreach_parallel)

        time.sleep(DURATION_S)

        def foreach_parallel(i, guest): # pyright: ignore[reportGeneralTypeIssues]
            guest.stop_ptp_client()
        end_foreach(guests, foreach_parallel)

        loadgen.stop_ptp4l()

        # remove unnecessary information
        def foreach_parallel(i, guest): # pyright: ignore[reportGeneralTypeIssues]
            guest.exec(f"grep -oP \"Delta between master and slave.*?:\K-?\d+\" {remote_path} | tail -n +2 | sudo tee {remote_path}.filtered")
            # guest.copy_from(f"{remote_path}.filtered", ptp_test.per_vm_output_path(repetition, i))
            guest.copy_from(f"{remote_path}.filtered", f"{ptp_test.per_vm_output_path(repetition, i)}.filtered")
            guest.copy_from(f"{remote_path}", ptp_test.per_vm_output_path(repetition, i))
        end_foreach(guests, foreach_parallel)

        with open(f"{ptp_test.output_filepath(repetition)}", "w") as f:
            f.write("done\n")

        pass


if __name__ == "__main__":
    measurement = Measurement()
    main(measurement)
