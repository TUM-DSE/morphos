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
    system: str # linux | unikraft

    def test_infix(self):
        return f"throughput_{self.interface}_{self.direction}_{self.vnf}_{self.size}B"

    def estimated_runtime(self) -> float:
        """
        estimate time needed to run this benchmark excluding boot time in seconds
        """
        overheads = 35
        return (self.repetitions * (DURATION_S + 2) ) + overheads

    def run_linux_tx(self, repetition: int, guest, loadgen, host):
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

    def run_linux_rx(self, repetition: int, guest, loadgen, host):
        loadgen.exec(f"sudo modprobe pktgen")
        pass

    def run_unikraft_tx(self, repetition: int, guest, loadgen, host, remote_unikraft_log_raw):
        pass

    def run_unikraft_rx(self, repetition: int, guest, loadgen, host, remote_unikraft_log_raw):
        remote_pktgen_log = "/tmp/pktgen.log"
        remote_unikraft_log = f"{remote_unikraft_log_raw}.{repetition}"
        local_unikraft_log = self.output_filepath(repetition)
        local_pktgen_log = self.output_filepath(repetition, extension="pktgen.log")

        loadgen.exec(f"sudo rm {remote_pktgen_log} || true")
        host.exec(f"sudo rm {remote_unikraft_log} || true")

        # start network load
        info("Starting pktgen")
        pktgen_cmd = f"{loadgen.project_root}/nix/builds/linux-pktgen/bin/pktgen_sample03_burst_single_flow" + \
            f" -i {host.test_bridge} -s {self.size} -d {guest.test_iface_ip_net} -m {guest.test_iface_mac} -b 1 | tee {remote_pktgen_log}; sleep 10";
        loadgen.tmux_kill("pktgen")
        loadgen.tmux_new("pktgen", pktgen_cmd)
        # reset unikraft log
        host.exec(f"sudo truncate -s 0 {remote_unikraft_log_raw}")

        time.sleep(DURATION_S)

        # copy raw to log, but only printable characters (cut leading null bytes)
        host.exec(f"strings {remote_unikraft_log_raw} | sudo tee {remote_unikraft_log}")
        loadgen.exec("sudo pkill -SIGINT pktgen")

        host.copy_from(remote_unikraft_log, local_unikraft_log)
        loadgen.copy_from(remote_pktgen_log, local_pktgen_log)
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
    systems = [ "linux", "unikraft" ]
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
        directions = [ "rx" ]
        # vm_nums = [ 1, 2, 4 ]
        vm_nums = [ 1 ]
        # vm_nums = [ 128, 160 ]
        DURATION_S = 10
        repetitions = 1
        vnfs = [ "empty" ]
        systems = [ "unikraft" ]

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
        system=systems,
    )
    tests: List[ThroughputTest] = []
    tests = ThroughputTest.list_tests(test_matrix, exclude_test=exclude)


    args_reboot = ["interface", "num_vms", "direction", "system"]
    info(f"Iperf Test execution plan:")
    ThroughputTest.estimate_time2(tests, args_reboot)

    if plan_only:
        return

    with Bench(
            tests = tests,
            args_reboot = args_reboot,
            brief = G.BRIEF
            ) as (bench, bench_tests):
        for [num_vms, interface, direction, system], a_tests in bench.multi_iterator(bench_tests, ["num_vms", "interface", "direction", "system"]):
            interface = Interface(interface)
            info("Booting VM for this test matrix:")
            info(ThroughputTest.test_matrix_string(a_tests))

            assert len(a_tests) == 1 # we have looped through all variables now, right?
            test = a_tests[0]
            info(f"Running {test}")

            if system == "unikraft":
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
                remote_unikraft_log_raw  = "/tmp/unikraft.log" # will be cleared sometimes
                remote_unikraft_init_log  = f"{remote_unikraft_log_raw}.init" # contains the startup log
                host.exec(f"sudo rm {remote_unikraft_log_raw} || true")
                host.exec(f"sudo rm {remote_unikraft_init_log} || true")
                with measurement.unikraft_vm(interface, click_config, remote_unikraft_log_raw) as guest:
                    host.exec(f"sudo cp {remote_unikraft_log_raw} {remote_unikraft_init_log}")
                    for repetition in range(repetitions):
                        if test.direction == "tx":
                            test.run_unikraft_tx(repetition, guest, loadgen, host, remote_unikraft_log_raw)
                        elif test.direction == "rx":
                            test.run_unikraft_rx(repetition, guest, loadgen, host, remote_unikraft_log_raw)
                # end VM

            elif system == "linux":
                # boot VMs
                with measurement.virtual_machine(interface) as guest:
                    for repetition in range(repetitions):

                        if test.direction == "tx":
                            test.run_linux_tx(repetition, guest, loadgen, host)
                        elif test.direction == "rx":
                            test.run_linux_rx(repetition, guest, loadgen, host)
                    pass
                # end VM

            else:
                raise ValueError(f"Unknown system: {system}")


            bench.done(test)


if __name__ == "__main__":
    measurement = Measurement()
    main(measurement)
