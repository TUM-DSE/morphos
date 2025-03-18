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

@dataclass
class ReconfigurationTest(AbstractBenchTest):

    vnf: str # workload
    system: str # linux | uk | ukebpfjit

    def test_infix(self):
        return f"reconfiguration_{self.system}_{self.vnf}"

    def estimated_runtime(self) -> float:
        """
        estimate time needed to run this benchmark excluding boot time in seconds
        """
        return 0


    def parse_results(self, repetition: int) -> DataFrame:
        values = []
        if self.system == "xdp":
            # parse output
            with open(self.output_filepath(repetition), 'r') as f:
                for line in f.readlines():
                    if line.startswith("real"):
                        time_str = line.split("\t")[1].strip()
                        time_s = float(time_str.split("m")[0]) * 60 + float(time_str.split("m")[1][:-1])
                        values += [ ("total", int(time_s*1000000000)) ]

        if self.system == "ukebpfjit":
            # parse output
            with open(self.output_filepath(repetition), 'r') as f:
                for line in f.readlines():
                    if line.startswith("Startup trace (nsec):"):
                        splits = line.split(":")
                        label = splits[1].strip()
                        value = splits[2].strip()
                        values += [ (label, int(value)) ]

        else:
            raise ValueError(f"Unknown system: {self.system}")

        data = []
        for (label, value) in values:
            data += [{
                **asdict(self), # put selfs member variables and values into this dict
                "repetition": repetition,
                "label": label,
                "nsec": value,
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
                files = [ "benchmark/bpfilters/target-port", "benchmark/bpfilters/target-port.sig" ]
                processing += "-> BPFilter(ID 1, FILE target-port, SIGNATURE target-port.sig, JIT false)"
            case ("ukebpfjit", "filter", _):
                files = [ "benchmark/bpfilters/target-port", "benchmark/bpfilters/target-port.sig" ]
                processing += "-> BPFilter(ID 1, FILE target-port, SIGNATURE target-port.sig, JIT true)"

            case ("ukebpf", "nat", "rx"):
                files = [ "benchmark/bpfilters/nat", "benchmark/bpfilters/nat.sig" ]
                processing += "rw :: BPFClassifier(ID 1, FILE nat, SIGNATURE nat.sig, JIT false)"
            case ("ukebpfjit", "nat", "rx"):
                files = [ "benchmark/bpfilters/nat", "benchmark/bpfilters/nat.sig" ]
                processing += "rw :: BPFClassifier(ID 1, FILE nat, SIGNATURE nat.sig, JIT true)"
            case (_, "nat", _):
                files = []
                processing += "rw :: IPRewriter(pattern NAT 0 1, pass 1);"

            case ("ukebpf", "mirror", "rx"):
                files = [ "benchmark/bpfilters/ether-mirror", "benchmark/bpfilters/ether-mirror.sig" ]
                processing += "-> BPFRewriter(ID 1, FILE ether-mirror, SIGNATURE ether-mirror.sig, JIT false)"
            case ("ukebpfjit", "mirror", "rx"):
                files = [ "benchmark/bpfilters/ether-mirror", "benchmark/bpfilters/ether-mirror.sig" ]
                processing += "-> BPFRewriter(ID 1, FILE ether-mirror, SIGNATURE ether-mirror.sig, JIT true)"
            case ("uk", "mirror", "rx"):
                files = []
                processing += "-> EtherMirror()" # this and its ebpf version should probably also do IPMirror()
            case ("linux", "mirror", "rx"):
                files = []
                processing += "-> EtherMirror() -> SimpleQueue(256)"

            case ("ukebpf", "ids", _):
                files = [ "benchmark/bpfilters/stringmatcher", "benchmark/bpfilters/stringmatcher.sig" ]
                processing += "-> BPFilter(ID 1, FILE stringmatcher, SIGNATURE stringmatcher.sig, JIT false)"
            case ("ukebpfjit", "ids", _):
                files = [ "benchmark/bpfilters/stringmatcher", "benchmark/bpfilters/stringmatcher.sig" ]
                processing += "-> BPFilter(ID 1, FILE stringmatcher, SIGNATURE stringmatcher.sig, JIT true)"
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
        self.start_pktgen_helper(guest, loadgen, host, pktgen_cmd)

    def start_pktgen_helper(self, guest, loadgen, host, pktgen_cmd):
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
        with open("/tmp/linux.click", "w") as text_file:
            text_file.write(config)
        guest.copy_to("/tmp/linux.click", "/tmp/linux.click")
        guest.start_click("/tmp/linux.click", remote_click_output, script_args=click_args, dpdk=False)

        info("Start measuring with bmon")
        # count packets that actually arrive, but cut first line because it is always zero
        monitor_cmd = f"bmon -p {loadgen.test_iface} -o '{bmon_format}' | tee {remote_monitor_file}"
        loadgen.tmux_kill("monitor")
        loadgen.tmux_new("monitor", monitor_cmd)

        time.sleep(G.DURATION_S)

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
        elif self.vnf == "mirror":
            config = click_configs.mirror(
                interface=guest.test_iface,
                ip=strip_subnet_mask(loadgen.test_iface_ip_net),
                mac=loadgen.test_iface_mac,
                extra_element=element
            )
        else:
            config = click_rx_config(guest.test_iface, extra_processing=element)
        with open("/tmp/linux.click", "w") as text_file:
            text_file.write(config)
        guest.copy_to("/tmp/linux.click", "/tmp/linux.click")
        guest.start_click("/tmp/linux.click", remote_click_output, script_args=click_args, dpdk=False)
        # start network load
        self.start_pktgen(guest, loadgen, host, remote_pktgen_log)

        time.sleep(G.DURATION_S)

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

        time.sleep(G.DURATION_S)

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

        time.sleep(G.DURATION_S)

        # copy raw to log, but only printable characters (cut leading null bytes)
        host.exec(f"strings {remote_unikraft_log_raw} | sudo tee {remote_unikraft_log}")
        # stop network load
        self.stop_pktgen(loadgen)

        host.copy_from(remote_unikraft_log, local_unikraft_log)
        loadgen.copy_from(remote_pktgen_log, local_pktgen_log)
        pass


def main(measurement: Measurement, plan_only: bool = False) -> None:
    host, loadgen = measurement.hosts()

    # set up test plan
    systems = [ "linux", "uk", "ukebpfjit" ]
    vm_nums = [ 1 ]
    vnfs = [ "empty", "filter", "nat", "ids", "mirror" ]
    repetitions = 3
    G.DURATION_S = 71 if not G.BRIEF else 15
    if G.BRIEF:
        # systems = [ "linux", "uk", "ukebpfjit" ]
        # systems = [ "uk", "ukebpfjit" ]
        systems = [ "ukebpfjit" ]
        # systems = [ "xdp" ]
        # systems = [ "linux" ]
        # vnfs = [ "empty" ]
        vnfs = [ "empty" ]
        repetitions = 1

    def exclude(test):
        return False

    test_matrix = dict(
        repetitions=[ repetitions ],
        num_vms=vm_nums,
        vnf=vnfs,
        system=systems,
    )
    tests: List[ReconfigurationTest] = []
    tests = ReconfigurationTest.list_tests(test_matrix, exclude_test=exclude)


    args_reboot = ["num_vms", "system", "vnf"]
    info(f"ThroughputTest execution plan:")
    ReconfigurationTest.estimate_time2(tests, args_reboot)

    if plan_only:
        return

    with Bench(
            tests = tests,
            args_reboot = args_reboot,
            brief = G.BRIEF
            ) as (bench, bench_tests):
        for [system, vnf], a_tests in bench.multi_iterator(bench_tests, ["system", "vnf"]):

            # info("Booting VM for this test matrix:")
            # info(ReconfigurationTest.test_matrix_string(a_tests))

            assert len(a_tests) == 1 # we have looped through all variables now, right?
            test = a_tests[0]
            info(f"Running {test}")


            for repetition in range(repetitions):
                if test.system == "ukebpfjit":
                    remote_qemu_log = "/tmp/qemu.log"
                    remote_test_done = "/tmp/test_done"
                    local_outfile = test.output_filepath(repetition)
                    dir = f"{host.project_root}/benchmark"

                    # clean old outfiles
                    host.exec(f"sudo rm {remote_qemu_log} || true")
                    host.exec(f"sudo rm {remote_test_done} || true")

                    # start test
                    env_vars = f"QEMU_OUT='{remote_qemu_log}' ONLY='pass (BPFFilter - JIT)'"
                    bench_cmd = "cargo bench --bench live_reconfigure"
                    cmd = f"cd {dir}; {env_vars} nix develop --command {bench_cmd}; echo done > {remote_test_done}"
                    host.tmux_new("qemu0", cmd)

                    # wait for test to complete
                    time.sleep(15) # by default, citerion.rs tries to run benchmarks for 5 seconds
                    try:
                        host.wait_for_success(f'[[ -e {remote_test_done} ]]', timeout=30)
                    except TimeoutError:
                        error('Waiting for fastclick output file to appear timed out')

                    # collect results
                    host.tmux_kill("qemu0")
                    host.copy_from(remote_qemu_log, local_outfile)


                elif test.system == "uk":
                    print("")
                elif test.system == "linux":
                    print("")
                elif test.system == "xdp":
                    iface = "eno1" # TODO
                    xdp_program = f"{host.project_root}/nix/builds/xdp/lib/reflector.o"
                    remote_outfile = "/tmp/xdp.log"
                    local_outfile = test.output_filepath(repetition)
                    iterations = 30
                    host.exec(f"sudo rm {remote_outfile} || true")
                    # time must be executed in sh. Other shells use other time impls
                    xdp_add_cmd = f"ip link set {iface} xdpgeneric obj {xdp_program} sec xdp"
                    xdp_del_cmd = f"sudo ip link set {iface} xdpgeneric off || true"
                    cmd = f"sudo /bin/sh -c \"for i in {{1..{iterations}}}; do {xdp_del_cmd}; time {xdp_add_cmd}; done\" 2>&1 | tee -a {remote_outfile}"
                    host.exec(cmd)
                    host.copy_from(remote_outfile, local_outfile)
            breakpoint()


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
    # all_data['mpps'] = all_data['pps'].apply(lambda pps: pps / 1_000_000)
    # del all_data["pps"]
    df = all_data.groupby([ col for col in all_data.columns if col != "nsec" ]).describe()
    with open(path_join(G.OUT_DIR, "reconfiguration_summary.log"), 'w') as file:
        file.write(df.to_string())


if __name__ == "__main__":
    measurement = Measurement()
    main(measurement)
