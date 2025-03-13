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
import click_configs
from conf import G
from measure_throughput import ThroughputTest, click_tx_config, click_rx_config

unikraft_interface = "0"
safe_vpp_warmup = False # without we rarely get excessive standard deviations
GUEST_IP = "10.10.0.1"
TEST_CLIENT_IP = "10.10.0.2"


@dataclass
class FirewallTest(ThroughputTest):

    # test options (additionally to ThroughputTest)
    fw_size: int # nr of firewall rules

    def test_infix(self):
        return f"firewall_{self.system}_{self.interface}_{self.direction}_{self.fw_size}_{self.size}B"

    def click_config(self) -> Tuple[List[str], str]:
        files = [] # relative to project root
        processing = ""

        rx_ip_check = """
            // stripping only makes sense, once we've looked at the ethernet header
            -> Classifier(12/0800)
            // check ip header doesn't want ethernet header
            -> Strip(14)
            // some elements like IPFilter segfault with some packets if we don't check them
            -> CheckIPHeader
        """
        if self.direction == "rx" and self.vnf == "filter":
            processing += rx_ip_check

        match (self.system, self.vnf, self.direction):
            case ("linux", "firewall", "rx"):
                files = []
                processing += "-> IPFilter(deny dst port 1234, allow all)" # TODO
            case ("uk", "firewall", "rx"):
                files = []
                processing += "-> IPFilter(deny dst port 1234, allow all)" # TODO
            case ("ukebpf", "firewall", "rx"):
                name = f"firewall-{self.fw_size}"
                files = [ f"benchmark/bpfilters/{name}", f"benchmark/bpfilters/{name}.sig" ]
                processing += f"-> BPFilter(ID 1, FILE {name}, SIGNATURE {name}.sig, JIT false)"
            case ("ukebpfjit", "firewall", "rx"):
                name = f"firewall-{self.fw_size}"
                files = [ f"benchmark/bpfilters/{name}", f"benchmark/bpfilters/{name}.sig" ]
                processing += f"-> BPFilter(ID 1, FILE {name}, SIGNATURE {name}.sig, JIT true)"

            case _:
                raise ValueError(f"Unknown system/vnf combination: {self.system}/{self.vnf}")

        return files, processing



def main(measurement: Measurement, plan_only: bool = False) -> None:
    host, loadgen = measurement.hosts()

    # set up test plan
    interfaces = [
          Interface.VPP,
          # Interface.BRIDGE_VHOST,
          ]
    directions = [ "rx" ]
    systems = [ "linux", "uk", "ukebpfjit" ]
    vm_nums = [ 1 ]
    sizes = [ 64 ]
    vnfs = [ "firewall" ]
    fw_sizes = [ 2, 100, 1000
                # , 10000 # click ebpf: jit target buffer too small
                ]
    repetitions = 3
    G.DURATION_S = 71 if not G.BRIEF else 15
    if safe_vpp_warmup:
        G.DURATION_S = max(30, G.DURATION_S)
    if G.BRIEF:
        # interfaces = [ Interface.BRIDGE ]
        # interfaces = [ Interface.BRIDGE_VHOST ]
        interfaces = [ Interface.VPP ]
        # interfaces = [ Interface.BRIDGE_VHOST, Interface.VPP ]
        # directions = [ "rx", "tx" ]
        directions = [ "rx" ]
        # systems = [ "linux", "uk", "ukebpfjit" ]
        # systems = [ "uk", "ukebpfjit" ]
        # systems = [ "uk" ]
        systems = [ "ukebpfjit" ]
        # systems = [ "linux" ]
        vm_nums = [ 1 ]
        # vm_nums = [ 128, 160 ]
        # vnfs = [ "empty" ]
        sizes = [ 64 ]
        vnfs = [ "firewall" ]
        # fw_sizes [ 2 ]
        fw_sizes = [ 2, 100, 1000 ]
        repetitions = 1

    test_matrix = dict(
        repetitions=[ repetitions ],
        direction=directions,
        interface=[ interface.value for interface in interfaces],
        num_vms=vm_nums,
        size=sizes,
        vnf=vnfs,
        system=systems,
        fw_size=fw_sizes,
    )
    tests: List[FirewallTest] = []
    tests = FirewallTest.list_tests(test_matrix)


    args_reboot = ["interface", "num_vms", "direction", "system", "vnf", "size", "fw_size"]
    info(f"FirewallTest execution plan:")
    FirewallTest.estimate_time2(tests, args_reboot)

    if plan_only:
        return

    with Bench(
            tests = tests,
            args_reboot = args_reboot,
            brief = G.BRIEF
            ) as (bench, bench_tests):
        for [num_vms, interface, direction, system, vnf, size, fw_size], a_tests in bench.multi_iterator(bench_tests, ["num_vms", "interface", "direction", "system", "vnf", "size", "fw_size"]):
            interface = Interface(interface)

            info("Booting VM for this test matrix:")
            info(FirewallTest.test_matrix_string(a_tests))

            assert len(a_tests) == 1 # we have looped through all variables now, right?
            test = a_tests[0]
            info(f"Running {test}")


            debug('Binding loadgen interface')
            loadgen.modprobe_test_iface_drivers()
            loadgen.release_test_iface() # bind linux driver
            try:
                loadgen.delete_nic_ip_addresses(loadgen.test_iface)
            except Exception:
                pass
            if test.vnf == "nat":
                loadgen.exec(f"sudo ip address add {TEST_CLIENT_IP}/32 dev {loadgen.test_iface}")
            loadgen.setup_test_iface_ip_net()


            if system in [ "uk", "ukebpf", "ukebpfjit" ]:
                files, element = test.click_config()
                click_config = ""
                if test.vnf == "nat":
                    click_config = click_configs.nat(
                        interface=unikraft_interface,
                        guest_ip=GUEST_IP,
                        guest_mac=measurement.guest.test_iface_mac,
                        gw_ip=strip_subnet_mask(loadgen.test_iface_ip_net),
                        gw_mac=loadgen.test_iface_mac,
                        src_ip=TEST_CLIENT_IP,
                        src_mac=loadgen.test_iface_mac,
                        dst_ip=strip_subnet_mask(loadgen.test_iface_ip_net),
                        dst_mac=measurement.guest.test_iface_mac,
                        size=test.size,
                        direction=test.direction,
                        rewriter=element
                    )
                elif test.vnf == "mirror":
                    click_config = click_configs.mirror(
                        interface=unikraft_interface,
                        ip=strip_subnet_mask(loadgen.test_iface_ip_net),
                        mac=loadgen.test_iface_mac,
                        extra_element=element
                    )
                elif test.direction == "tx":
                    click_config = click_tx_config(unikraft_interface, size=test.size, dst_mac=loadgen.test_iface_mac, extra_processing=element)
                elif test.direction == "rx":
                    click_config = click_rx_config(unikraft_interface, extra_processing=element)

                remote_unikraft_log_raw  = "/tmp/unikraft.log" # will be cleared sometimes
                remote_unikraft_init_log  = f"{remote_unikraft_log_raw}.init" # contains the startup log
                host.exec(f"sudo rm {remote_unikraft_log_raw} || true")
                host.exec(f"sudo rm {remote_unikraft_init_log} || true")

                for repetition in range(repetitions): # restarting click for each repetition means restarting unikraft
                    with measurement.unikraft_vm(interface, click_config, vm_log=remote_unikraft_log_raw, cpio_files=files) as guest:
                        host.exec(f"sudo cp {remote_unikraft_log_raw} {remote_unikraft_init_log}")

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

    # parse all results
    all_dfs = []
    for test in tests:
        for repetition in range(test.repetitions):
            local_csv_file = test.output_filepath(repetition, "csv")
            with open(local_csv_file, 'w') as file:
                try:
                    df = test.parse_results(repetition)
                    raw_data = df.to_csv()
                    all_dfs += [ df ]
                except Exception as e:
                    raw_data = str(e)
                    error(f"Error parsing results of {local_csv_file}: {e}")
                file.write(raw_data)

    # summarize results
    all_data = pd.concat(all_dfs)
    del all_data["repetition"]
    all_data['mpps'] = all_data['pps'].apply(lambda pps: pps / 1_000_000)
    del all_data["pps"]
    df = all_data.groupby([ col for col in all_data.columns if col != "mpps" ]).describe()
    with open(path_join(G.OUT_DIR, f"throughput_summary.log"), 'w') as file:
        file.write(df.to_string())


if __name__ == "__main__":
    measurement = Measurement()
    main(measurement)
