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

unikraft_interface = "0"
safe_vpp_warmup = False # without we rarely get excessive standard deviations
GUEST_IP = "10.10.0.1"
TEST_CLIENT_IP = "10.10.0.2"

def click_tx_config(interface: str, size: int = 1460, dst_mac: str = "90:e2:ba:c3:76:6e", extra_processing: str = "") -> str:
    # size - 4 bytes ethernet CRC - 14 ethernet header - 20 IP header - 20 ethernet peamble maybe?
    size = size - 4 - 14 - 20 - 20
    return f"""
//Default values for packet length, number of packets and amountfs of time to replay them
define($L 60, $R 0, $S 100000);

//You do not need to change these to the real ones, just have the dmac match the receiver's one
define($mymac 90:e2:ba:c3:79:66)
define($dmac {dst_mac})
//Ip are just for a convenient payload as this is l2
define($myip 192.168.130.13)
define($dstip 192.168.128.13)

//Explained in loop.click
define($verbose 3)
define($blocking true)


InfiniteSource(DATA \<0800>, LENGTH {size}, LIMIT -1, BURST 100000)
// InfiniteSource2(DATA \<0800>, SRCIP $myip, DSTIP $dstip, LENGTH {size}, LIMIT -1, BURST 100000)
 -> UDPIPEncap($myip, 5678, $dstip, 5678)
{extra_processing}
-> EtherEncap(0x0800, $mymac, $dmac)
// FastUDPFlows(RATE 0, LIMIT -1, LENGTH {size}, SRCETH $mymac, DSTETH $dmac, SRCIP $myip, DSTIP $dstip, FLOWS 1, FLOWSIZE 100000)
-> ic0 :: AverageCounter()
-> ToDevice({interface});

fd :: FromDevice({interface}) -> Print('rx') -> Discard

Script(TYPE ACTIVE,
       wait 5ms,
       label start,
       print "Number of packets sent : $(ic0.rate)",
       wait 1s,
       goto start
       )
"""

def click_rx_config(interface: str, extra_processing: str = "") -> str:
    return f"""
from :: FromDevice({interface})
{extra_processing}
-> ic0 :: AverageCounter()
-> Discard;

Script(TYPE ACTIVE,
       wait 5ms,
       label start,
       print "Rx rate: $(ic0.rate)",
       write ic0.reset 1,
       write from.reset 1,
       wait 1s,
       goto start
       )
    """

bmon_format = "format:fmt=\$(attr:rxrate:packets)\t\$(attr:rxrate:bytes)\n"

@dataclass
class ThroughputTest(AbstractBenchTest):

    # test options
    direction: str # VM's point of view: rx | tx

    interface: str # network interface used
    size: int # packet size
    vnf: str # workload
    system: str # linux | uk | ukebpf | ukebpfjit

    def test_infix(self):
        return f"throughput_{self.system}_{self.interface}_{self.direction}_{self.vnf}_{self.size}B"

    def pktgen_output_filepath(self, repetition: int) -> str:
        return self.output_filepath(repetition, extension="pktgen.log")

    def estimated_runtime(self) -> float:
        """
        estimate time needed to run this benchmark excluding boot time in seconds
        """
        overheads = 5
        return (self.repetitions * (DURATION_S + 2) ) + overheads


    def parse_results(self, repetition: int) -> DataFrame:
        warmup = 4 # how many seconds to skip at the beginning
        if safe_vpp_warmup and Interface(self.interface).needs_vpp():
            warmup = 10 # vpp implements active queue management which delays packet bursts

        values = []
        if self.direction == "rx":
            # parse click log
            with open(self.output_filepath(repetition), 'r') as f:
                lines = f.readlines()
            lines = [ line for line in lines if "Rx rate: " in line ]
            # pps values
            values = [ float(line.split("Rx rate: ")[1].strip().split(" ")[0].strip()) for line in lines ]
            values = values[warmup:]

        elif self.direction == "tx":
            # parse click log
            with open(self.output_filepath(repetition), 'r') as f:
                lines = f.readlines()
            # pps values
            values = [ float(line.split("\t")[0].strip()) for line in lines ]
            values = values[warmup:]

        else:
            raise ValueError(f"Unknown direction: {self.direction}")

        data = []
        for value in values:
            data += [{
                **asdict(self), # put selfs member variables and values into this dict
                "repetition": repetition,
                "pps": value,
            }]
        return DataFrame(data=data)


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
            case (_, "empty", _):
                files = []
                processing += ""

            case ("linux", "filter", "rx"):
                files = []
                processing += "-> IPFilter(deny dst port 1234, allow all)" # push/pull mismatch for tx
            case ("linux", "filter", "tx"):
                files = []
                processing += "-> IPFilter2(deny dst port 1234, allow all)" # push/pull mismatch for tx
            case ("uk", "filter", _):
                files = []
                processing += "-> IPFilter(deny dst port 1234, allow all)"
            case ("ukebpf", "filter", _):
                files = [ "benchmark/bpfilters/stringmatcher", "benchmark/bpfilters/stringmatcher.sig" ]
                processing += "-> BPFilter(ID 1, FILE stringmatcher, SIGNATURE stringmatcher.sig, JIT false)"
            case ("ukebpfjit", "filter", _):
                files = [ "benchmark/bpfilters/target-port", "benchmark/bpfilters/target-port.sig" ]
                processing += "-> BPFilter(ID 1, FILE target-port, SIGNATURE target-port.sig, JIT true)"
            case ("uk", "nat", _):
                files = []
                # dont append, but replace processing because we use another config template
                processing = "rw :: IPRewriter(pattern NAT 0 1, pass 1);"
            case ("ukebpf", "nat", _):
                files = [ "benchmark/bpfilters/nat", "benchmark/bpfilters/nat.sig" ]
                # dont append, but replace processing because we use another config template
                processing = "rw :: BPFClassifier(ID 1, FILE nat, SIGNATURE nat.sig, JIT false)"
            case ("ukebpfjit", "nat", _):
                files = [ "benchmark/bpfilters/nat", "benchmark/bpfilters/nat.sig" ]
                # dont append, but replace processing because we use another config template
                processing = "rw :: BPFClassifier(ID 1, FILE nat, SIGNATURE nat.sig, JIT true)"
            case (_, "ids", _):
                files = []
                processing += "-> StringMatcher(teststringtomatch)"

            case _:
                raise ValueError(f"Unknown system/vnf combination: {self.system}/{self.vnf}")

        return files, processing

    def start_pktgen(self, guest, loadgen, host, remote_pktgen_log):
        info("Starting pktgen")

        batch = 32
        threads = 2

        # when doing localhost measurements, pktgen attaches to virtual devices.
        # They don't support batching and likely only have 1 queue (thread support).
        # Therefore, we enable fast pktgen only for non-localhost measurements.
        if host.fqdn == "localhost":
            batch = 1
            threads = 1

        size = self.size - 4 # subtract 4 bytes for the CRC
        pktgen_cmd = f"{loadgen.project_root}/nix/builds/linux-pktgen/bin/pktgen_sample03_burst_single_flow" + \
            f" -i {loadgen.test_iface} -s {self.size - 4} -d {strip_subnet_mask(guest.test_iface_ip_net)} -m {guest.test_iface_mac} -b {batch} -t {threads} | tee {remote_pktgen_log}";
        loadgen.tmux_kill("pktgen")

        # sometimes, pktgen returns immediately with 0 packets sent
        # i believe this happens, when the link is not ready due to peer reset
        retries = 10
        for i in range(retries + 1):
            if i >= retries:
                raise Exception("Failed to start pktgen")

            # start pktgen
            loadgen.tmux_new("pktgen", pktgen_cmd)

            time.sleep(1)
            if loadgen.tmux_is_alive("pktgen"):
                break
            debug("Pktgen exited immediately, restarting")

    def stop_pktgen(self, loadgen):
        loadgen.exec("sudo pkill -SIGINT pktgen")

    def run_linux_tx(self, repetition: int, guest, loadgen, host):
        remote_monitor_file = "/tmp/throughput.tsv"
        remote_click_output = "/tmp/click.log"
        local_monitor_file = self.output_filepath(repetition)
        local_click_output = self.output_filepath(repetition, "click.log")

        loadgen.exec(f"sudo rm {remote_monitor_file} || true")
        guest.exec(f"sudo rm {remote_click_output} || true")

        click_args = { "R": 0 }
        guest.kill_click()
        _, element = self.click_config()
        config = click_tx_config(guest.test_iface, size=self.size, dst_mac=loadgen.test_iface_mac, extra_processing=element)
        guest.write(config, "/tmp/linux.click")
        guest.start_click("/tmp/linux.click", remote_click_output, script_args=click_args, dpdk=False)

        info("Start measuring with bmon")
        # count packets that actually arrive, but cut first line because it is always zero
        monitor_cmd = f"bmon -p {loadgen.test_iface} -o '{bmon_format}' | tee {remote_monitor_file}"
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

        remote_click_output = "/tmp/click.log"
        remote_pktgen_log = "/tmp/pktgen.log"
        local_click_output = self.output_filepath(repetition)
        local_pktgen_log = self.pktgen_output_filepath(repetition)

        loadgen.exec(f"sudo rm {remote_pktgen_log} || true")
        guest.exec(f"sudo rm {remote_click_output} || true")

        click_args = {}
        guest.kill_click()
        _, element = self.click_config()
        if self.vnf == "nat":
            config = click_configs.nat(
                interface=guest.test_iface,
                guest_ip=GUEST_IP,
                guest_mac=measurement.guest.test_iface_mac,
                gw_ip=strip_subnet_mask(loadgen.test_iface_ip_net),
                gw_mac=loadgen.test_iface_mac,
                src_ip=TEST_CLIENT_IP,
                src_mac=loadgen.test_iface_mac,
                dst_ip=strip_subnet_mask(loadgen.test_iface_ip_net),
                dst_mac=measurement.guest.test_iface_mac,
                size=self.size,
                direction=self.direction,
                rewriter=element
            )
        else:
            config = click_rx_config(guest.test_iface, extra_processing=element)
        guest.write(config, "/tmp/linux.click")
        guest.start_click("/tmp/linux.click", remote_click_output, script_args=click_args, dpdk=False)
        # start network load
        self.start_pktgen(guest, loadgen, host, remote_pktgen_log)

        time.sleep(DURATION_S)

        # stop network load
        self.stop_pktgen(loadgen)

        guest.stop_click()
        guest.kill_click()

        loadgen.copy_from(remote_pktgen_log, local_pktgen_log)
        guest.copy_from(remote_click_output, local_click_output)


    def run_unikraft_tx(self, repetition: int, guest, loadgen, host, remote_unikraft_log_raw):
        remote_monitor_file = "/tmp/throughput.tsv"
        remote_unikraft_log = f"{remote_unikraft_log_raw}.{repetition}"
        local_monitor_file = self.output_filepath(repetition)
        local_unikraft_log = self.output_filepath(repetition, "unikraft.log")

        loadgen.exec(f"sudo rm {remote_monitor_file} || true")
        host.exec(f"sudo rm {remote_unikraft_log} || true")

        # reset unikraft log
        host.exec(f"sudo truncate -s 0 {remote_unikraft_log_raw}")

        info("Start measuring with bmon")
        monitor_cmd = f"bmon -p {loadgen.test_iface} -o '{bmon_format}' | tee {remote_monitor_file}"
        loadgen.tmux_kill("monitor")
        loadgen.tmux_new("monitor", monitor_cmd)

        time.sleep(DURATION_S)

        loadgen.tmux_kill("monitor")

        # copy raw to log, but only printable characters (cut leading null bytes)
        host.exec(f"strings {remote_unikraft_log_raw} | sudo tee {remote_unikraft_log}")
        host.copy_from(remote_unikraft_log, local_unikraft_log)
        loadgen.copy_from(remote_monitor_file, local_monitor_file)
        pass

    def run_unikraft_rx(self, repetition: int, guest, loadgen, host, remote_unikraft_log_raw):
        loadgen.exec(f"sudo modprobe pktgen")

        remote_pktgen_log = "/tmp/pktgen.log"
        remote_unikraft_log = f"{remote_unikraft_log_raw}.{repetition}"
        local_unikraft_log = self.output_filepath(repetition)
        local_pktgen_log = self.pktgen_output_filepath(repetition)

        loadgen.exec(f"sudo rm {remote_pktgen_log} || true")
        host.exec(f"sudo rm {remote_unikraft_log} || true")

        # start network load
        self.start_pktgen(guest, loadgen, host, remote_pktgen_log)
        # reset unikraft log
        host.exec(f"sudo truncate -s 0 {remote_unikraft_log_raw}")

        time.sleep(DURATION_S)

        # copy raw to log, but only printable characters (cut leading null bytes)
        host.exec(f"strings {remote_unikraft_log_raw} | sudo tee {remote_unikraft_log}")
        # stop network load
        self.stop_pktgen(loadgen)

        host.copy_from(remote_unikraft_log, local_unikraft_log)
        loadgen.copy_from(remote_pktgen_log, local_pktgen_log)
        pass


def main(measurement: Measurement, plan_only: bool = False) -> None:
    host, loadgen = measurement.hosts()
    global DURATION_S

    # set up test plan
    interfaces = [
          Interface.VPP,
          Interface.BRIDGE_VHOST,
          ]
    directions = [ "rx", "tx" ]
    systems = [ "linux", "uk", "ukebpfjit" ]
    vm_nums = [ 1 ]
    sizes = [ 64, 256, 1024, 1518 ]
    vnfs = [ "empty", "filter", "nat", "ids" ]
    repetitions = 3
    DURATION_S = 71 if not G.BRIEF else 15
    if safe_vpp_warmup:
        DURATION_S = max(30, DURATION_S)
    if G.BRIEF:
        # interfaces = [ Interface.BRIDGE ]
        interfaces = [ Interface.BRIDGE_VHOST ]
        # interfaces = [ Interface.VPP ]
        # interfaces = [ Interface.BRIDGE_VHOST, Interface.VPP ]
        directions = [ "rx" ]
        # systems = [ "linux", "uk", "ukebpfjit" ]
        # systems = [ "ukebpfjit" ]
        systems = [ "linux" ]
        vm_nums = [ 1 ]
        # vm_nums = [ 128, 160 ]
        # vnfs = [ "empty" ]
        sizes = [ 64 ]
        vnfs = [ "nat" ]
        repetitions = 1

    def exclude(test):
        return ((Interface(test.interface).is_passthrough() and test.num_vms > 1) or
                    (test.vnf == "nat" and test.direction == "tx") # packets get stuck in queue
        )

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


    args_reboot = ["interface", "num_vms", "direction", "system", "vnf", "size"]
    info(f"ThroughputTest execution plan:")
    ThroughputTest.estimate_time2(tests, args_reboot)

    if plan_only:
        return

    with Bench(
            tests = tests,
            args_reboot = args_reboot,
            brief = G.BRIEF
            ) as (bench, bench_tests):
        for [num_vms, interface, direction, system, vnf, size], a_tests in bench.multi_iterator(bench_tests, ["num_vms", "interface", "direction", "system", "vnf", "size"]):
            interface = Interface(interface)

            info("Booting VM for this test matrix:")
            info(ThroughputTest.test_matrix_string(a_tests))

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
