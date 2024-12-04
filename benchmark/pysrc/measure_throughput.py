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
from util import safe_cast, product_dict, strip_subnet_mask, deduplicate
from typing import Iterator, cast, List, Dict, Callable, Tuple, Any
import time
from os.path import isfile, join as path_join
import yaml
import json
from root import *
from dataclasses import dataclass, field, asdict
import subprocess
import os
from pandas import DataFrame
import pandas as pd
import traceback
from conf import G



@dataclass
class ThroughputTest(AbstractBenchTest):

    # test options
    direction: str # VM's point of view: rx | tx

    interface: str # network interface used
    size: int # packet size
    vnf: str # workload

    def test_infix(self):
        return f"throughput_{self.interface}_{self.direction}_{self.vnf}_{self.size}B"

    def estimated_runtime(self) -> float:
        """
        estimate time needed to run this benchmark excluding boot time in seconds
        """
        overheads = 35
        return (self.repetitions * (DURATION_S + 2) ) + overheads

    def run_tx(self, repetition: int, guest, loadgen, host):
        remote_monitor_file = "/tmp/throughput.tsv"
        remote_click_output = "/tmp/click.log"
        local_monitor_file = self.output_filepath(repetition)
        local_click_output = self.output_filepath(repetition, "click.log")

        loadgen.exec(f"sudo rm {remote_monitor_file} || true")
        guest.exec(f"sudo rm {remote_click_output} || true")

        click_args = { "R": 0 }
        guest.kill_click()
        guest.start_click("benchmark/configurations/linux-tx.click", remote_click_output, script_args=click_args, dpdk=False)
        # count packets that actually arrive, but cut first line because it is always zero
        monitor_cmd = f"bmon -p {host.test_tap} -o 'format:fmt=\$(attr:rxrate:packets)\t\$(attr:rxrate:bytes)\n' | tee {remote_monitor_file}"
        loadgen.tmux_kill("monitor")
        loadgen.tmux_new("monitor", monitor_cmd)
        time.sleep(DURATION_S)
        loadgen.tmux_kill("monitor")
        guest.stop_click()
        guest.kill_click()

        loadgen.copy_from(remote_monitor_file, local_monitor_file)
        guest.copy_from(remote_click_output, local_click_output)

    def run_rx(self, repetition: int, guest, loadgen, host):
        loadgen.exec(f"sudo modprobe pktgen")
        pass



def main(measurement: Measurement, plan_only: bool = False) -> None:
    host, loadgen = measurement.hosts()
    global DURATION_S

    # set up test plan
    interfaces = [
          Interface.VFIO,
          Interface.BRIDGE,
          Interface.BRIDGE_VHOST,
          Interface.BRIDGE_E1000,
          # Interface.VMUX_PT, # interrupts dont work
          Interface.VMUX_EMU,
          # Interface.VMUX_EMU_E810, # tap backend not implemented for e810 (see #127)
          # Interface.VMUX_DPDK, # e1000 dpdk backend doesn't support multi-VM
          Interface.VMUX_DPDK_E810,
          Interface.VMUX_MED
          ]
    udp_interfaces = [ # tap based interfaces have very broken ARP behaviour with udp
          Interface.VFIO,
          Interface.VMUX_DPDK,
          Interface.VMUX_DPDK_E810,
          Interface.VMUX_MED
          ]
    directions = [ "forward" ]
    vm_nums = [ 1, 2, 4, 8, 16, 32, 64 ]
    sizes = [ 64 ]
    vnfs = [ "empty" ]
    repetitions = 3
    DURATION_S = 61 if not G.BRIEF else 11
    if G.BRIEF:
        interfaces = [ Interface.BRIDGE ]
        # interfaces = [ Interface.VMUX_DPDK_E810, Interface.BRIDGE_E1000 ]
        # interfaces = [ Interface.VMUX_MED ]
        # interfaces = [ Interface.VMUX_EMU ]
        directions = [ "tx" ]
        # vm_nums = [ 1, 2, 4 ]
        vm_nums = [ 1 ]
        # vm_nums = [ 128, 160 ]
        DURATION_S = 10
        repetitions = 1
        vnfs = [ "empty" ]

    def exclude(test):
        return (Interface(test.interface).is_passthrough() and test.num_vms > 1)

    # multi-VM TCP tests, but only one length
    test_matrix = dict(
        repetitions=[ repetitions ],
        direction=directions,
        interface=[ interface.value for interface in interfaces],
        num_vms=vm_nums,
        size=sizes,
        vnf=vnfs,
    )
    tests: List[ThroughputTest] = []
    tests = ThroughputTest.list_tests(test_matrix, exclude_test=exclude)


    args_reboot = ["interface", "num_vms", "direction"]
    info(f"Iperf Test execution plan:")
    ThroughputTest.estimate_time2(tests, args_reboot)

    if plan_only:
        return

    with Bench(
            tests = tests,
            args_reboot = args_reboot,
            brief = G.BRIEF
            ) as (bench, bench_tests):
        for [num_vms, interface, direction], a_tests in bench.multi_iterator(bench_tests, ["num_vms", "interface", "direction"]):
            interface = Interface(interface)
            info("Booting VM for this test matrix:")
            info(ThroughputTest.test_matrix_string(a_tests))

            click_config = """
            FromDevice(0)
            -> ic0 :: AverageCounter()
            -> Discard;

            Script(TYPE ACTIVE,
 				            wait 5ms,
				            label start,
				            print "Rx rate: $(ic0.count)",
				            write ic0.reset 1,
				            wait 1s,
				            goto start
				            )
            """
            host.exec("sudo rm /tmp/unikraft.log || true")
            with measurement.unikraft_vm(interface, click_config, "/tmp/unikraft.log"):
                breakpoint()
                pass

            # boot VMs
            with measurement.virtual_machine(interface) as guest:
                assert len(a_tests) == 1 # we have looped through all variables now, right?
                test = a_tests[0]
                info(f"Running {test}")

                for repetition in range(repetitions):

                    if test.direction == "tx":
                        test.run_tx(repetition, guest, loadgen, host)
                    elif test.direction == "rx":
                        test.run_rx(repetition, guest, loadgen, host)

                bench.done(test)

                # info('Binding loadgen interface')
                # loadgen.modprobe_test_iface_drivers()
                # loadgen.release_test_iface() # bind linux driver

                # try:
                #     loadgen.delete_nic_ip_addresses(loadgen.test_iface)
                # except Exception:
                #     pass
                # loadgen.setup_test_iface_ip_net()
                # loadgen.stop_xdp_pure_reflector()
                # # loadgen.start_xdp_pure_reflector()
                # # install inter-VM ARP rules (except first one which actually receives ARP. If we prevent ARP on the first one, all break somehow.)
                # loadgen.add_arp_entries({ i_:guest_ for i_, guest_ in guests.items() if i_ != 1 })

                # def foreach_parallel(i, guest): # pyright: ignore[reportGeneralTypeIssues]
                #     guest.modprobe_test_iface_drivers(interface=interface)
                #     guest.setup_test_iface_ip_net()
                # end_foreach(guests, foreach_parallel)

                # for [proto, length], b_tests in bench.multi_iterator(a_tests, ["proto", "length"]):
                #         assert len(b_tests) == 1 # we have looped through all variables now, right?
                #         test = b_tests[0]
                #         info(f"Running {test}")

                #         for repetition in range(repetitions):
                #             test.run(repetition, guests, loadgen, host)

                #         bench.done(test)
                # loadgen.stop_xdp_pure_reflector()
            # end VM


if __name__ == "__main__":
    measurement = Measurement()
    main(measurement)
