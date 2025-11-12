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
import measure_throughput
import measure_firewall

unikraft_interface = "0"

def bpftrace_program(which_qemu):
    return """
        tracepoint:kvm:kvm_entry / @a[pid] == 0 / { printf(\\"qemu kvm entry ns %lld\\n\\", nsecs()); @a[pid] = 1; }
        tracepoint:kvm:kvm_pio / args.port == 0xf4 / { printf(\\"qemu kvm port %d ns %lld\\n\\", args->val, nsecs()); }
        tracepoint:syscalls:sys_enter_execve*
        / str(args.filename) == \\"WHICH_QEMU\\" /
        { printf(\\"qemu start ns %lld\\n\\", nsecs()); printf(\\"filename %s\\n\\", str(args.filename)); }
    """.replace("WHICH_QEMU", which_qemu)

def click_rx_config(from_device: str, extra_processing: str = "") -> str:
    return f"""
{from_device}
{extra_processing}
-> ic0 :: AverageCounter()
-> Discard;

Script(TYPE ACTIVE,
       print "sleeping first increases startup time"
       wait 5ms,
       label start,
       print "Rx rate: $(ic0.rate)",
       wait 1s,
       goto start
       )
    """

@dataclass
class ReconfigurationTest(AbstractBenchTest):

    vnf: str # workload
    system: str # linux | uk | uktrace | ukebpfjit

    def test_infix(self):
        return f"reconfiguration_{self.system}_{self.vnf}"

    def estimated_runtime(self) -> float:
        """
        estimate time needed to run this benchmark excluding boot time in seconds
        """
        return 10 * self.repetitions


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

        elif self.system in [ "uk", "uktrace", "ukebpfjit", "linux" ]:
            # parse output
            with open(self.output_filepath(repetition), 'r') as f:
                for line in f.readlines():
                    if line.startswith("Startup trace (nsec):"):
                        splits = line.split(":")
                        label = splits[1].strip()
                        value = splits[2].strip()
                        values += [ (label, int(value)) ]

                    if line.startswith("Received packet from device:"):
                        value = int(line.split(":")[1].split("ns")[0].strip())
                        values += [ ("first packet", value) ]


                    if line.startswith("Bench-helper startup time (nsec):"):
                        value = int(line.split(":")[1].strip())
                        values += [ ("total startup time", value) ]

            if self.system == "uktrace":
                with open(self.output_filepath(repetition, extension="bpftrace.log"), 'r') as f:
                    for line in f.readlines():
                        if "qemu" in line and "ns" in line:
                            splits = line.split("ns")
                            label = splits[0].strip()
                            value = int(splits[1].strip())
                            values += [ (label, value) ]

            if self.system == "ukebpfjit":
                data = pd.read_csv(self.output_filepath(repetition, extension="criterion.csv"))
                for index, row in data.iterrows():
                    assert row["unit"] == "ns"
                    iterations = int(row["iteration_count"])
                    value = int(int(row["sample_measured_value"]) / float(iterations))
                    values += [ ("total", value) for _ in range(iterations) ]

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

    def click_config(self) -> str:
        if self.system == "linux":
            from_device = "from :: KernelTun(172.44.0.2/24)"
        else:
            from_device = "from :: FromDevice(0)"

        if self.system == "uktrace":
            system = "uk"
        else:
            system = self.system

        if "firewall" in self.vnf:
            fw_size = int(self.vnf.split("-")[1])
            reference_test = measure_firewall.FirewallTest(direction="rx", interface="any", size=0, repetitions=0, num_vms=1, vnf="firewall", system=system, fw_size=fw_size)
            (files, element) = reference_test.click_config()
        else:
            reference_test = measure_throughput.ThroughputTest(direction="rx", interface="any", size=0, repetitions=0, num_vms=1, vnf=self.vnf, system=system)
            (files, element) = reference_test.click_config()
        assert len(files) == 0
        element = f" -> Print2('Received packet from device') {element}"
        config = click_rx_config(from_device, extra_processing=element)
        return config



def main(measurement: Measurement, plan_only: bool = False) -> None:
    host, loadgen = measurement.hosts()

    # set up test plan
    systems = [
        # we define these manually below:
        "linux",
        "uk",
        "uktrace",
        "ukebpfjit",
        "xdp"
    ]
    vm_nums = [ 1 ]
    vnfs = [ "empty", "filter", "ids", "mirror", "nat", "firewall-2", "firewall-1000" ]
    repetitions = 3
    iterations = 10 # some benchmarks need to be told how often to run before returning
    G.DURATION_S = 0
    if G.BRIEF:
        # systems = [ "linux", "uk", "ukebpfjit" ]
        # systems = [ "uk", "ukebpfjit" ]
        # systems = [ "ukebpfjit" ]
        # systems = [ "uk", "uktrace", "linux" ]
        # systems = [ "xdp" ]
        systems = [ "uktrace" ]
        # systems = [ "linux" ]
        vnfs = [ "empty" ]
        # vnfs = [ "nat" ]
        # vnfs = [ "firewall-10000" ]
        # vnfs = [ "empty", "filter", "ids", "mirror", "nat", "firewall-2" ]
        repetitions = 1
        iterations = 1

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
    if not G.BRIEF:
        tests += [ ReconfigurationTest(repetitions=repetitions, num_vms=1, vnf="reflector", system="xdp") ]
        # tests += [ ReconfigurationTest(repetitions=repetitions, num_vms=1, vnf="nat", system="linux") ]
        # tests += [ ReconfigurationTest(repetitions=repetitions, num_vms=1, vnf="nat", system="uk") ]
        # tests += [ ReconfigurationTest(repetitions=repetitions, num_vms=1, vnf="nat", system="uktrace") ]


    args_reboot = ["num_vms", "system", "vnf"]
    info(f"ReconfigurationTest execution plan:")
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

            host.exec(f"cd {PROJECT_ROOT}; nix develop --command make -C benchmark setup")
            subprocess.run(["sudo", "rm", "/tmp/linux.click"], check=False) # clean up other user's tmpfile on localhost

            for repetition in range(repetitions):
                if test.system == "ukebpfjit":
                    # cargo bench --bench live_reconfigure
                    dir = f"{host.project_root}/benchmark"
                    criterion_selector = f"{test.vnf}-jit"
                    remote_qemu_log = "/tmp/qemu.log"
                    remote_test_done = "/tmp/test_done"
                    local_outfile = test.output_filepath(repetition)
                    remote_criterion_file = f"{dir}/target/criterion/live-reconfigure/{criterion_selector}/new/raw.csv"
                    local_criterion_file = test.output_filepath(repetition, extension="criterion.csv")

                    # clean old outfiles
                    host.exec(f"sudo rm {remote_qemu_log} || true")
                    host.exec(f"sudo rm {remote_test_done} || true")
                    host.exec(f"sudo rm '{remote_criterion_file}' || true")

                    # start test
                    env_vars = f"QEMU_OUT='{remote_qemu_log}' ONLY='{criterion_selector}'"
                    bench_cmd = "cargo bench --bench live_reconfigure"
                    cmd = f"cd {dir}; {env_vars} nix develop --command {bench_cmd}; echo done > {remote_test_done}"
                    host.tmux_kill("qemu")
                    host.tmux_new("qemu", cmd)

                    # wait for test to complete
                    time.sleep(15) # by default, citerion.rs tries to run benchmarks for 5 seconds
                    try:
                        host.wait_for_success(f'[[ -e {remote_test_done} ]]', timeout=30)
                    except TimeoutError:
                        error('Waiting for test to finish timed out')

                    # collect results
                    host.tmux_kill("qemu")
                    host.copy_from(remote_qemu_log, local_outfile)
                    host.copy_from(remote_criterion_file, local_criterion_file)

                elif test.system in [ "linux", "uk", "uktrace" ]:
                    # cargo run --bin bench-helper --features print-output
                    remote_qemu_log = "/tmp/qemu.log"
                    remote_bpftrace_log = "/tmp/bpftrace.log"
                    remote_test_done = "/tmp/test_done"
                    local_outfile = test.output_filepath(repetition)
                    local_tracefile = test.output_filepath(repetition, extension="bpftrace.log")
                    dir = f"{host.project_root}"
                    def nix_prefix(env_vars="", subdir=""):
                        return f"cd {dir}/{subdir}; {env_vars} nix develop --command"
                    which_qemu = host.exec(f"{nix_prefix()} which qemu-system-x86_64")
                    which_qemu = which_qemu.strip().splitlines()[-1]

                    # clean old stuff
                    host.exec(f"sudo rm {remote_qemu_log} || true")
                    host.tmux_kill("bpftrace")
                    host.tmux_kill("qemu")
                    if test.system == "uktrace":
                        host.exec(f"sudo rm {remote_bpftrace_log} || true")
                        env_vars = "BPFTRACE_MAX_STRLEN=123"
                        bpftrace_cmd = f"{nix_prefix(env_vars=env_vars)} sudo -E bpftrace -e '{bpftrace_program(which_qemu)}' 2>&1 | tee {remote_bpftrace_log}; sleep 999"
                        host.tmux_new("bpftrace", bpftrace_cmd)
                        host.wait_for_success(f'[[ $(tail -n 5 {remote_bpftrace_log}) = *"Attaching"* ]]')

                    for iteration in range(iterations):
                        host.exec(f"sudo rm {remote_test_done} || true")

                        # start test
                        match (test.system, test.vnf):
                            case ("linux", "nat"):
                                env_vars = "ONLY=linux-thomer-nat"
                            case ("uk", "nat"):
                                env_vars = "ONLY=uk-thomer-nat"
                            case ("uktrace", "nat"):
                                env_vars = "ONLY=uk-thomer-nat"
                            case (_, _):
                                config = test.click_config()
                                with open("/tmp/linux.click", "w") as text_file:
                                    text_file.write(config)
                                host.exec("sudo rm /tmp/config.click || true")
                                host.copy_to("/tmp/linux.click", "/tmp/config.click")
                                if test.system == "uktrace":
                                    env_vars = "ONLY=uk"
                                else:
                                    env_vars = f"ONLY={test.system}"
                        bench_cmd = "cargo run --bin bench-helper --features print-output"
                        cmd = f"{nix_prefix(env_vars=env_vars, subdir='benchmark')} {bench_cmd} 2>&1 | tee -a {remote_qemu_log}; echo done > {remote_test_done}"
                        host.tmux_kill("qemu")
                        host.tmux_new("qemu", cmd)

                        # wait for test to complete
                        time.sleep(3)
                        try:
                            host.wait_for_success(f'[[ -e {remote_test_done} ]]', timeout=60)
                        except TimeoutError:
                            error('Waiting for test to finish timed out')
                        host.tmux_kill("qemu")

                    # collect results
                    host.copy_from(remote_qemu_log, local_outfile)
                    if test.system == "uktrace":
                        host.tmux_kill("bpftrace")
                        host.copy_from(remote_bpftrace_log, local_tracefile)

                elif test.system == "xdp":
                    iface = "eno1" # TODO
                    xdp_program = f"{host.project_root}/nix/builds/xdp/lib/reflector.o"
                    remote_outfile = "/tmp/xdp.log"
                    local_outfile = test.output_filepath(repetition)
                    iterations_many = iterations * 3
                    host.exec(f"sudo rm {remote_outfile} || true")
                    # time must be executed in sh. Other shells use other time impls
                    xdp_add_cmd = f"ip link set {iface} xdpgeneric obj {xdp_program} sec xdp"
                    xdp_del_cmd = f"sudo ip link set {iface} xdpgeneric off || true"
                    cmd = f"sudo /bin/sh -c \"for i in {{1..{iterations_many}}}; do {xdp_del_cmd}; time {xdp_add_cmd}; done\" 2>&1 | tee -a {remote_outfile}"
                    host.exec(cmd)
                    host.copy_from(remote_outfile, local_outfile)

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
