import autotest as autotest
from configparser import ConfigParser
from argparse import (ArgumentParser, ArgumentDefaultsHelpFormatter, Namespace,
                      FileType, ArgumentTypeError)
from argcomplete import autocomplete
from logging import (info, debug, error, warning, getLogger,
                     DEBUG, INFO, WARN, ERROR)
from server import Host, Guest, LoadGen
from enums import Machine, Interface, Reflector, MultiHost
from measure import Bench, AbstractBenchTest, Measurement, end_foreach
from util import safe_cast, product_dict
from typing import Iterator, cast, List, Dict, Callable, Tuple, Any
import time
from os.path import isfile, join as path_join
import yaml
import json
from root import *
from dataclasses import dataclass, field, asdict
import subprocess
import os
from pandas import DataFrame, read_csv, concat
from conf import G

PER_VM_FLOWS = 3

@dataclass
class MediationTest(AbstractBenchTest):

    # test options
    interface: str # network interface used
    fastclick: str # one of: software | hardware
    rate: int # offered load, kpps

    def test_infix(self):
        return f"mediation_{self.fastclick}_{self.num_vms}vms_{self.interface}_{self.rate}kpps"

    def output_filepath_histogram(self, repetition: int) -> str:
        return self.output_filepath(repetition, extension = "histogram.csv")

    def output_filepath_throughput(self, repetition: int) -> str:
        return self.output_filepath(repetition, extension = "throughput.csv")

    def output_filepath_fastclick(self, repetition: int, vm_number: int) -> str:
        return str(Path(G.OUT_DIR) / f"mediationVMs_{self.test_infix()}_rep{repetition}" / f"vm{vm_number}.log")

    def estimated_runtime(self) -> float:
        """
        estimate time needed to run this benchmark excluding boot time in seconds
        """
        time_sleeps = DURATION_S + 10 + 5
        overheads = 35
        return self.repetitions * (time_sleeps + overheads)

    # def find_error(self, repetition: int) -> bool:
    #     failure = False
    #     if not self.output_json:
    #         file = self.output_filepath(repetition)
    #         if os.stat(file).st_size == 0:
    #             error(f"Some iperf tests returned errors:\n{file}")
    #             failure = True
    #     return failure

    def summarize(self, repetition: int, vm_num: int) -> DataFrame:
        with open(self.output_filepath_fastclick(repetition, vm_num), 'r') as f:
            fastclickLog: List[str] = f.readlines()
        with open(self.output_filepath_throughput(repetition), 'r') as f:
            moongen = read_csv(f)

        # parse fastclickLog
        packet_count_lines = [line for line in fastclickLog if "thread " in line and " count:" in line]
        packet_counts = [int(line.strip().split(' ')[-1]) for line in packet_count_lines]
        rx_packets = sum(packet_counts)

        # parse moongen
        tx_packets = moongen[moongen.Direction=='TX'].tail(1)['TotalPackets'].iloc[0]
        tx_mpps = moongen[moongen.Direction=='TX'].tail(1)['PacketRate'].iloc[0]

        packet_loss = (tx_packets - rx_packets) / rx_packets
        rx_mpps_calc = (rx_packets / tx_packets) * tx_mpps

        data = [{
            **asdict(self), # put selfs member variables and values into this dict
            "repetition": repetition,
            "vm_num": vm_num,
            "rxPackets": rx_packets,
            "txPackets": tx_packets,
            "txMpps": tx_mpps,
            "rxMppsCalc": rx_mpps_calc,
        }]
        return DataFrame(data=data)



def strip_subnet_mask(ip_addr: str):
    return ip_addr[ : ip_addr.index("/")]


# rules for software fastclick
def fastclick_mac_classifiers(vm_number: int):
    ret = dict()
    start = (vm_number - 1) * PER_VM_FLOWS
    for i in range(PER_VM_FLOWS):
        offset = start + i
        mac = MultiHost.mac("00:00:00:00:00:00", offset)
        class__ = "".join(reversed(mac.split(":")))
        class_ = f"0/{class__}"
        ret[f"class{i}"] = class_
    return ret

# rules for software fastclick
def fastclick_ethertype_classifiers(vm_number: int):
    ret = dict()
    start = 0x1234
    for i in range(PER_VM_FLOWS):
        etype = start + i
        class_ = f"0/{etype:X}"
        ret[f"class{i}"] = class_
    return ret

# rules for rte_flow fastclick
def write_fastclick_rules(guest: Guest, vm_number: int, path: str):
    def line(mac: str, queue: int):
        return f"ingress pattern eth dst spec {mac} dst mask ff:ff:ff:ff:ff:ff / end actions queue index {queue} / end\n"

    start = (vm_number - 1) * PER_VM_FLOWS
    content = ""
    for i in range(PER_VM_FLOWS):
        offset = start + i
        mac = MultiHost.mac("00:00:00:00:00:00", offset)
        content += line(mac, i)
    guest.write(content, path)


def run_test(host: Host, loadgen: LoadGen, guests: Dict[int, Guest], test: MediationTest, repetition: int):
    example_guest = next(iter(guests.values()))
    remote_fastclick_log = "/tmp/fastclick.log"
    remote_moongen_throughput = "/tmp/throughput.csv"
    local_moongen_throughput = test.output_filepath_throughput(repetition)
    remote_moongen_histogram = "/tmp/histogram.csv"
    local_moongen_histogram = test.output_filepath_histogram(repetition)
    def foreach_parallel(i, guest): # pyright: ignore[reportGeneralTypeIssues]
        guest.exec(f"sudo rm {remote_fastclick_log} || true")
    end_foreach(guests, foreach_parallel)
    loadgen.exec(f"sudo rm {remote_moongen_throughput} || true")
    loadgen.exec(f"sudo rm {remote_moongen_histogram} || true")
    loadgen.stop_l2_load_latency(loadgen)

    info("Starting Fastclick")
    if test.fastclick == "software":
        fastclick_program = "test/fastclick/mac-switch-software.click"
        fastclick_args = {
            'ifacePCI0': example_guest.test_iface_addr,
        }
    elif test.fastclick == "hardware":
        fastclick_program = "test/fastclick/mac-switch-hardware.click"
        project_root = str(Path(example_guest.moonprogs_dir) / "../..") # click wants nicely formatted paths
        fastclick_rules = f"{project_root}/test/fastclick/rteflow_etype_rules"
        # fastclick_rules = f"{project_root}/test/fastclick/rteflow_empty"
        # fastclick_rules = "/tmp/test_dpdk_nic_rules"
        # def foreach_parallel(i, guest): # pyright: ignore[reportGeneralTypeIssues]
        #     write_fastclick_rules(guest, i, fastclick_rules)
        # end_foreach(guests, foreach_parallel)
        fastclick_args = {
            'ifacePCI0': example_guest.test_iface_addr,
            'rules': fastclick_rules
        }
    elif test.fastclick == "software-tap":
        fastclick_program = "test/fastclick/mac-switch-software-tap.click"
        fastclick_args = {
            'if': example_guest.test_iface,
        }
    else:
        raise Exception("Unknown fastclick program type")

    def foreach_parallel(i, guest): # pyright: ignore[reportGeneralTypeIssues]
        args = { **fastclick_args, **fastclick_ethertype_classifiers(i) }
        guest.start_fastclick(fastclick_program, remote_fastclick_log, script_args=args)
    end_foreach(guests, foreach_parallel)
    time.sleep(10) # fastclick takes roughly as long as moongen to start, be we give it some slack nevertheless
    info("Starting MoonGen")
    loadgen.run_l2_load_latency(loadgen, "52:54:00:fa:00:61", test.rate,
            runtime = DURATION_S,
            size = 60,
            nr_macs = len(guests.values()),
            nr_ethertypes=PER_VM_FLOWS,
            histfile = remote_moongen_histogram,
            statsfile = remote_moongen_throughput,
            outfile = '/tmp/output.log'
            )
    time.sleep(DURATION_S + 5)

    # await moongen done
    try:
        loadgen.wait_for_success(f'[[ -e {remote_moongen_histogram} ]]')
    except TimeoutError:
        error('Waiting for moongen output file timed out.')

    # await fastclick logs
    def foreach_parallel(i, guest): # pyright: ignore[reportGeneralTypeIssues]
        guest.stop_fastclick()
        try:
            guest.wait_for_success(f'[[ $(tail -n 5 {remote_fastclick_log}) = *"AUTOTEST_DONE"* ]]')
        except TimeoutError:
            error('Waiting for fastclick output file to appear timed out')
        guest.kill_fastclick()
    end_foreach(guests, foreach_parallel)

    # teardown
    loadgen.stop_l2_load_latency(loadgen)

    # collect artefacts
    def foreach_parallel(i, guest): # pyright: ignore[reportGeneralTypeIssues]
        guest.copy_from(remote_fastclick_log, test.output_filepath_fastclick(repetition, i))
    end_foreach(guests, foreach_parallel)
    loadgen.copy_from(remote_moongen_throughput, local_moongen_throughput)
    loadgen.copy_from(remote_moongen_histogram, local_moongen_histogram)

    # breakpoint()
    pass


def exclude(test):
    return (Interface(test.interface).is_passthrough() and test.num_vms > 1) or \
        (Interface(test.interface) == Interface.VMUX_DPDK and test.fastclick == "software" and test.num_vms > 1)

        # (Interface(test.interface) == Interface.VMUX_DPDK_E810 and test.num_vms > 1) or \


def main(measurement: Measurement, plan_only: bool = False) -> None:
    host, loadgen = measurement.hosts()
    global DURATION_S

    # set up test plan
    interfaces = [
          Interface.VMUX_MED,
          Interface.VMUX_DPDK_E810,
          Interface.VFIO,
          Interface.VMUX_PT,
          ]
    tap_interfaces = [
          Interface.BRIDGE_E1000,
          Interface.BRIDGE,
          Interface.BRIDGE_VHOST,
          Interface.VMUX_DPDK,
          ]
    fastclicks = [
            "hardware",
            "software",
            ]
    vm_nums = [ 1, 2, 4, 8, 16, 32, 64 ] # 2 VMs are currently not supported for VMUX_DPDK*
    rates = [ 40000 ]
    repetitions = 3
    DURATION_S = 61 if not G.BRIEF else 11
    if G.BRIEF:
        # interfaces = [ Interface.BRIDGE_E1000 ] # dpdk doesnt bind (not sure why)
        # interfaces = [ Interface.BRIDGE ] # doesnt work with click-dpdk (RSS init fails)
        interfaces = [ Interface.VMUX_MED ]
        # interfaces = [ Interface.VMUX_DPDK_E810 ]
        # interfaces = [ Interface.VMUX_PT ]
        fastclicks = [ "hardware" ]
        # fastclicks = [ "hardware" ]
        # fastclicks = [ "software-tap" ]
        vm_nums = [ 4 ]
        repetitions = 1
        # DURATION_S = 10000
        rates = [ 40000 ]

    tests = []
    test_matrix = dict(
        repetitions=[ repetitions ],
        interface=[ interface.value for interface in interfaces],
        fastclick = fastclicks,
        num_vms=vm_nums,
        rate=rates
    )
    tests += MediationTest.list_tests(test_matrix, exclude_test=exclude)

    # software-tap benchmarks
    test_matrix = dict(
        repetitions=[ repetitions ],
        interface=[ interface.value for interface in tap_interfaces ],
        fastclick = [ "software-tap" ],
        num_vms=vm_nums,
        rate=rates
    )
    if not G.BRIEF:
        tests += MediationTest.list_tests(test_matrix, exclude_test=exclude)

    info(f"Mediation test execution plan:")
    args_reboot = ["interface", "num_vms", "repetitions", "fastclick"]
    MediationTest.estimate_time2(tests, args_reboot = args_reboot)


    if plan_only:
        return

    with Bench(
            tests = tests,
            args_reboot = args_reboot,
            brief = G.BRIEF
            ) as (bench, bench_tests):
        for [num_vms, interface, fastclick], a_tests in bench.multi_iterator(bench_tests, ["num_vms", "interface", "fastclick"]):
            for repetition in range(repetitions):
                interface = Interface(interface)
                info("Booting VM for this test matrix:")
                info(MediationTest.test_matrix_string(a_tests))
                with measurement.virtual_machines(interface, num_vms) as guests:
                    for rate, rate_tests in bench.iterator(a_tests, "rate"):
                        assert len(rate_tests) == 1 # we have looped through all variables now, right?
                        test = rate_tests[0]

                        info(f"Testing repetition {repetition}: {test}")

                        # loadgen: set up interfaces and networking

                        info('Binding loadgen interface')
                        try:
                            loadgen.delete_nic_ip_addresses(loadgen.test_iface)
                        except Exception:
                            pass
                        loadgen.modprobe_test_iface_drivers()
                        loadgen.bind_test_iface() # bind vfio driver

                        # guest: set up networking

                        def foreach_parallel(i, guest): # pyright: ignore[reportGeneralTypeIssues]
                            if interface.guest_driver() == "ice":
                                guest.bind_test_iface() # bind vfio driver
                            else: # supports software-tap with kernel interfaces only
                                guest.modprobe_test_iface_drivers(interface=interface)
                                guest.setup_test_iface_ip_net()
                        end_foreach(guests, foreach_parallel)

                        # run test
                        run_test(host, loadgen, guests, test, repetition)

                        # summarize results
                        local_summary_file = test.output_filepath(repetition)
                        with open(local_summary_file, 'w') as file:
                            dfs = []
                            for i, guest in guests.items():
                                try:
                                    # to_string preserves all cols
                                    dfs += [ test.summarize(repetition, i) ]
                                except Exception as e:
                                    summary = str(e)
                            summary = concat(dfs).to_string()
                            file.write(summary)
                        bench.done(test)


    # summarize all summaries
    all_summaries = []
    for test in tests:
        for repetition in range(test.repetitions):
            all_summaries += [ read_csv(test.output_filepath(repetition), sep='\\s+') ]
    df = concat(all_summaries).groupby(MediationTest.test_parameters())['rxMppsCalc'].describe()

    with open(path_join(G.OUT_DIR, f"mediation_summary.log"), 'w') as file:
        file.write(df.to_string())

    # for ipt in all_tests:
    #     for repetition in range(repetitions):
    #         ipt.find_error(repetition)


if __name__ == "__main__":
    measurement = Measurement()
    main(measurement)
