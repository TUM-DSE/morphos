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
class IPerfTest(AbstractBenchTest):

    # test options
    direction: str # can be: forward | reverse | bidirectional
    output_json = True # sets / unsets -J flag on iperf client

    interface: str # network interface used
    length: int # iperf3 -l buffer write length (default 128000 for tcp, 9000 for udp)
    proto: str # tcp | udp

    def test_infix(self):
        return f"iperf3_{self.num_vms}vms_{self.interface}_{self.direction}_{self.proto}_{self.length}B"

    def output_path_per_vm(self, repetition: int, vm_number: int) -> str:
        return str(Path(G.OUT_DIR) / f"iperf3VMs_{self.test_infix()}_rep{repetition}" / f"vm{vm_number}.json")

    def estimated_runtime(self) -> float:
        """
        estimate time needed to run this benchmark excluding boot time in seconds
        """
        overheads = 35
        return (self.repetitions * (DURATION_S + 2) ) + overheads

    def find_error(self, repetition: int) -> bool:
        failure = False
        if not self.output_json:
            file = self.output_filepath(repetition)
            if os.stat(file).st_size == 0:
                error(f"Some iperf tests returned errors:\n{file}")
                failure = True
        return failure

    def summarize(self, repetition: int, vm_num: int) -> DataFrame:
        with open(self.output_path_per_vm(repetition, vm_num), 'r') as f:
            data = json.load(f)

        # Extract important values
        start = data['start']
        end = data['end']
        intervals = data['intervals']

        duration_secs = end['sum_sent']['seconds']
        sent_bytes = end['sum_sent']['bytes']
        sent_bits_per_second = end['sum_sent']['bits_per_second']
        received_bytes = end['sum_received']['bytes']
        received_bits_per_second = end['sum_received']['bits_per_second']

        gbitps = received_bits_per_second / 1024 / 1024 / 1024

        data = [{
            **asdict(self), # put selfs member variables and values into this dict
            "repetition": repetition,
            "vm_num": vm_num,
            "GBit/s": gbitps,
        }]
        return DataFrame(data=data)

    def run(self, repetition: int, guests, loadgen, host):
        #cleanup
        def foreach_parallel(i, guest): # pyright: ignore[reportGeneralTypeIssues]
            guest.stop_iperf_server()
            loadgen.stop_iperf_client(vm_num=i)
        end_foreach(guests, foreach_parallel)

        def remote_output_file(vm_num):
            return f"/tmp/iperf_result_vm{vm_num}.json"
        def tmp_remote_output_file(vm_num):
            return f"/tmp/tmp_iperf_result_vm{vm_num}.json"
        local_output_file = self.output_filepath(repetition)
        def foreach_parallel(i, guest): # pyright: ignore[reportGeneralTypeIssues]
            loadgen.exec(f"sudo rm {remote_output_file(i)} || true")
            loadgen.exec(f"sudo rm {tmp_remote_output_file(i)} || true")
        end_foreach(guests, foreach_parallel)

        info("Starting iperf")
        def foreach_parallel(i, guest): # pyright: ignore[reportGeneralTypeIssues]
            # workaround for ARP being broken in vMux: ping the loadgen once
            guest.wait_for_success(f"ping -c 1 -W 1 {strip_subnet_mask(loadgen.test_iface_ip_net)}")

            guest.start_iperf_server(strip_subnet_mask(guest.test_iface_ip_net))
        end_foreach(guests, foreach_parallel)
        def foreach_parallel(i, guest): # pyright: ignore[reportGeneralTypeIssues]
            loadgen.run_iperf_client(self, DURATION_S, strip_subnet_mask(guest.test_iface_ip_net), remote_output_file(i), tmp_remote_output_file(i), proto=self.proto, length=self.length, vm_num=i)
        end_foreach(guests, foreach_parallel)

        time.sleep(DURATION_S)
        # time.sleep(DURATION_S / 2)
        # remote_cpufile = "/tmp/vmux_cpupins.log"
        # host.exec(f"ps H -o pid,tid,%cpu,psr,comm -p $(pgrep vmux) > {remote_cpufile}; ps H -o pid,tid,%cpu,psr,args -p $(pgrep qemu) | awk '\"'\"'{{print $1, $2, $3, $4, $42}}'\"'\"' >> {remote_cpufile}")
        # time.sleep(DURATION_S / 2)

        def foreach_parallel(i, guest): # pyright: ignore[reportGeneralTypeIssues]
            try:
                loadgen.wait_for_success(f'[[ -e {remote_output_file(i)} ]]', timeout=30)
            except TimeoutError:
                error('Waiting for output file timed out.')
            finally:
                loadgen.copy_from(remote_output_file(i), self.output_path_per_vm(repetition, i))
        end_foreach(guests, foreach_parallel)
        # host.copy_from(remote_cpufile, self.output_filepath(repetition, extension="cpus"))


        # teardown
        def foreach_parallel(i, guest): # pyright: ignore[reportGeneralTypeIssues]
            guest.stop_iperf_server()
            loadgen.stop_iperf_client(vm_num=i)
        end_foreach(guests, foreach_parallel)

        # summarize results of VM
        with open(local_output_file, 'w') as file:
            dfs = []
            for i, guest in guests.items():
                try:
                    dfs += [ self.summarize(repetition, i) ]
                except Exception as e:
                    warning(f"Can't process result of VM {i} repetition {repetition}. Did the benchmark fail?")
                    _ignore = traceback.format_exc()
            # to_string preserves all cols
            if len(dfs) > 0:
                summary = pd.concat(dfs).to_string()
            else:
                summary = "no results"
            file.write(summary)



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
    tcp_lengths = [ -1 ]
    udp_lengths = [ 64, 128, 256, 512, 1024, 1470 ]
    repetitions = 3
    DURATION_S = 61 if not G.BRIEF else 11
    if G.BRIEF:
        # interfaces = [ Interface.BRIDGE_E1000 ]
        # interfaces = [ Interface.VMUX_DPDK_E810, Interface.BRIDGE_E1000 ]
        interfaces = [ Interface.VMUX_MED ]
        # interfaces = [ Interface.VMUX_EMU ]
        directions = [ "forward" ]
        # vm_nums = [ 1, 2, 4 ]
        vm_nums = [ 2 ]
        # vm_nums = [ 128, 160 ]
        DURATION_S = 10
        repetitions = 1

    def exclude(test):
        return (Interface(test.interface).is_passthrough() and test.num_vms > 1)

    tests: List[IPerfTest] = []

    # multi-VM TCP tests, but only one length
    test_matrix = dict(
        repetitions=[ repetitions ],
        direction=directions,
        interface=[ interface.value for interface in interfaces],
        num_vms=vm_nums,
        length=tcp_lengths,
        proto=["tcp"]
    )
    tests += IPerfTest.list_tests(test_matrix, exclude_test=exclude)

    if not G.BRIEF:
        # packet-size UDP tests, but only one VM and not tap based interfaces
        test_matrix = dict(
            repetitions=[ repetitions ],
            direction=directions,
            interface=[ interface.value for interface in udp_interfaces],
            num_vms=[ 1 ],
            length=udp_lengths,
            proto=["udp"]
        )
        tests += IPerfTest.list_tests(test_matrix, exclude_test=exclude)
        tests = deduplicate(tests)

    args_reboot = ["interface", "num_vms", "direction"]
    info(f"Iperf Test execution plan:")
    IPerfTest.estimate_time2(tests, args_reboot)

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
            info(IPerfTest.test_matrix_string(a_tests))

            # boot VMs
            with measurement.virtual_machines(interface, num_vms) as guests:
                # loadgen: set up interfaces and networking

                info('Binding loadgen interface')
                loadgen.modprobe_test_iface_drivers()
                loadgen.release_test_iface() # bind linux driver

                try:
                    loadgen.delete_nic_ip_addresses(loadgen.test_iface)
                except Exception:
                    pass
                loadgen.setup_test_iface_ip_net()
                loadgen.stop_xdp_pure_reflector()
                # loadgen.start_xdp_pure_reflector()
                # install inter-VM ARP rules (except first one which actually receives ARP. If we prevent ARP on the first one, all break somehow.)
                loadgen.add_arp_entries({ i_:guest_ for i_, guest_ in guests.items() if i_ != 1 })

                def foreach_parallel(i, guest): # pyright: ignore[reportGeneralTypeIssues]
                    guest.modprobe_test_iface_drivers(interface=interface)
                    guest.setup_test_iface_ip_net()
                end_foreach(guests, foreach_parallel)

                for [proto, length], b_tests in bench.multi_iterator(a_tests, ["proto", "length"]):
                        assert len(b_tests) == 1 # we have looped through all variables now, right?
                        test = b_tests[0]
                        info(f"Running {test}")

                        for repetition in range(repetitions):
                            test.run(repetition, guests, loadgen, host)

                        bench.done(test)
                loadgen.stop_xdp_pure_reflector()
            # end VM

    for ipt in tests:
        for repetition in range(repetitions):
            ipt.find_error(repetition)


if __name__ == "__main__":
    measurement = Measurement()
    main(measurement)
