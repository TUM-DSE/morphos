from typing import Iterator, cast, List, Dict, Callable, Tuple, Any
from measure import Bench, AbstractBenchTest, Measurement, end_foreach
from logging import (info, debug, error, warning, getLogger,
                     DEBUG, INFO, WARN, ERROR)
from dataclasses import dataclass, field, asdict
from conf import G
from root import *


@dataclass
class MiscTest(AbstractBenchTest):

    name: str # name of misc test

    def test_infix(self):
        return f"misc_{self.name}"

    def estimated_runtime(self) -> int:
        if self.name == "imagesize":
            return 5 * 60
        elif self.name == "safetytime":
            return 3 * 60
        else:
            raise ValueError(f"Unknown misc test {self.name}")

def main(measurement: Measurement, plan_only: bool = False) -> None:
    host, loadgen = measurement.hosts()

    test_matrix = dict(
        name=["imagesize", "safetytime"],
        repetitions=[1],
        num_vms=[1],
    )
    tests: List[MiscTest] = []
    tests += MiscTest.list_tests(test_matrix)

    args_reboot = []
    info(f"MiscTest execution plan:")
    MiscTest.estimate_time2(tests, args_reboot)

    if plan_only:
        return

    with Bench(
            tests = tests,
            args_reboot = args_reboot,
            brief = G.BRIEF
            ) as (bench, bench_tests):
        for [name], a_tests in bench.multi_iterator(bench_tests, ["name"]):

            assert len(a_tests) == 1 # we have looped through all variables now, right?
            test = a_tests[0]
            info(f"Running {test}")

            if name == "imagesize":
                repetition = 0
                remote_outfile = "/tmp/imagesizes.csv"
                remote_logfile = "/tmp/just.log"
                host.exec(f"sudo rm {remote_outfile} {remote_logfile} || true")
                local_outfile = test.output_filepath(repetition)
                host.exec(f"cd {PROJECT_ROOT}; nix develop --command bash -c 'just imagesizes > {remote_outfile}' 2> {remote_logfile}")
                host.copy_from(remote_outfile, local_outfile)
            elif name == "safetytime":
                repetition = 0
                remote_outfile = "/tmp/buildtime.csv"
                remote_logfile = "/tmp/just.log"
                host.exec(f"sudo rm {remote_outfile} {remote_logfile} || true")
                local_outfile = test.output_filepath(repetition)
                host.exec(f"cd {PROJECT_ROOT}; nix develop --command bash -c 'just nat_buildtime {remote_outfile}' 2> {remote_logfile}")
                host.copy_from(remote_outfile, local_outfile)
            else:
                raise ValueError(f"Unknown misc test {name}")

            bench.done(test)


if __name__ == "__main__":
    measurement = Measurement()
    main(measurement)
