import autotest as autotest
from configparser import ConfigParser
from argparse import (ArgumentParser, ArgumentDefaultsHelpFormatter, Namespace,
                      FileType, ArgumentTypeError)
from argcomplete import autocomplete
from logging import (info, debug, error, warning,
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
from dataclasses import dataclass, field
import subprocess
from conf import G

DURATION_S: int


@dataclass
class DeathStarBenchTest(AbstractBenchTest):
    app: str # e.g. hotelReservation
    rps: int # requests per second
    interface: str # network interface used

    def test_infix(self):
        return f"{self.app}_{self.interface}_{self.num_vms}vms_{self.rps}rps"

    def estimated_runtime(self) -> float:
        """
        estimate time needed to run this benchmark excluding boottime in seconds
        """
        app_setup = 0 # diff "running DeathStarBenchTest" "Running wrk2 x times"
        measurements = self.repetitions * DURATION_S
        if self.app == "hotelReservation":
            app_setup = (self.num_vms / 19) * 60 * 1
        if self.app == "socialNetwork":
            app_setup = (self.num_vms / 27) * 60 * 1.5
        if self.app == "mediaMicroservices":
            app_setup = (self.num_vms / 27) * 60 * 2 # this value is estimated, not measured

        return measurements + app_setup


class DeathStarBench:
    app: str
    docker_image_tar: str
    docker_compose: Dict[int, str] # vm id -> strings of content of docker-compose.yamls
    service_map: Dict[int, str] # vm id -> service name
    extra_hosts: str = "" # /etc/hosts format
    frontend: str # service name of frontend
    frontend_url: str = "host:port/api"
    script: str # path to wrk2 lua script

    def __init__(self, app: str, moonprogs_dir: str):
        assert app in [
            "hotelReservation",
            "socialNetwork",
            "mediaMicroservices",
        ] # others are not implemented

        self.app = app
        self.docker_image_tar = f"{moonprogs_dir}/../../VMs/docker-images-{self.app}.tar"

        self.docker_compose = {}
        self.service_map = {}
        self.parse_docker_compose()

        if self.app == "hotelReservation":
            self.frontend = "frontend"
            frontend_ip = MultiHost.ip("10.1.0.20", self.vm_number_of(self.frontend))
            self.frontend_url = f"{frontend_ip}:5000"
            self.script = "./wrk2/scripts/hotel-reservation/mixed-workload_type_1.lua"
        if self.app == "socialNetwork":
            self.frontend = "nginx-thrift"
            frontend_ip = MultiHost.ip("10.1.0.20", self.vm_number_of(self.frontend))
            self.frontend_url = f"{frontend_ip}:8080"
            self.script = "./wrk2/scripts/social-network/mixed-workload.lua"
        if self.app == "mediaMicroservices":
            self.frontend = "nginx-web-server"
            frontend_ip = MultiHost.ip("10.1.0.20", self.vm_number_of(self.frontend))
            self.frontend_url = f"{frontend_ip}:8080/wrk2-api/review/compose"
            self.script = "./wrk2/scripts/media-microservices/compose-review.lua"

    @staticmethod
    def find_errors(out_dir: str) -> bool:
        """
          Socket errors: connect 0, read 0, write 0, timeout 156
          Non-2xx or 3xx responses: 529
        """
        failure = False
        out = ""
        try:
            match = "Non-2xx or 3xx responses"
            out = subprocess.check_output(["grep", "-r", match, out_dir])
        except subprocess.CalledProcessError:
            failure = True

        errors_found = not failure
        if errors_found:
            error(f"Some wrk2 tests returned errors:\n{out}")
        return errors_found

    def parse_docker_compose(self):
        # create per-VM docker-compose
        with open(f"{PROJECT_ROOT}/subprojects/deathstarbench/{self.app}/docker-compose.yml", "r") as file:
            docker_compose_full = yaml.safe_load(file)
        version = docker_compose_full["version"]
        vm_number = 1
        for service in docker_compose_full["services"].keys():
            docker_compose = dict()
            # docker_compose["version"] = f'{version} '
            docker_compose["services"] = dict()
            docker_compose["services"][service] = docker_compose_full["services"][service]
            docker_compose["services"][service]["network_mode"] = "host" # --network host
            docker_compose["services"][service]["depends_on"] = dict() # ignore stated dependencies (cant be resolved because they are not local)
            docker_compose["services"][service]["restart"] = "on-failure"
            if self.app == "hotelReservation":
                docker_compose["services"][service]["volumes"] = [ "./config.json:/config.json" ]

            self.docker_compose[vm_number] = str(yaml.dump(docker_compose))
            self.service_map[vm_number] = service
            self.extra_hosts += f"{MultiHost.ip('192.168.56.20', vm_number)} {service}\n"
            vm_number += 1

    def docker_compose_cleanup(self, guest: Guest):
        guest.exec("docker-compose down || true")
        # obsolete volumes quickly fill up VM images
        guest.exec("docker volume prune -f || true")
        # maybe also docker compose rm and docker volume rm

    def install_configs(self, guest: Guest):
        guest.update_extra_hosts(self.extra_hosts)
        guest.exec("rm -r ./* || true")
        local_dir = f"{PROJECT_ROOT}/subprojects/deathstarbench/{self.app}/*"
        guest.copy_to(local_dir, "./", recursive=True)

    def vm_number_of(self, service_name: str) -> int:
        try:
            ret = list(filter(lambda i: i[1] == service_name, self.service_map.items()))[0][0]
            return ret
        except IndexError:
            raise Exception(f"DeathStarBench.vm_number_of: service {service_name} not found")

    def run(self, host: Host, loadgen: LoadGen, guests: Dict[int, Guest], test: DeathStarBenchTest):
        assert test.app == self.app

        # setup docker-compose env
        def foreach_parallel(i, guest): # pyright: ignore[reportGeneralTypeIssues]
            self.docker_compose_cleanup(guest)
            self.install_configs(guest)
            guest.write(self.docker_compose[i], "docker-compose.yml")
            guest.exec(f"docker load -i {self.docker_image_tar}")
        end_foreach(guests, foreach_parallel)

        # start docker-compose

        def foreach_parallel(i, guest): # pyright: ignore[reportGeneralTypeIssues]
            guest.exec(f"docker-compose up -d")
        end_foreach(guests, foreach_parallel)

        # complete setup

        connections = 16
        if self.app == "socialNetwork":
            frontend_guest = guests[self.vm_number_of(self.frontend)]
            if not frontend_guest.test("[[ -f /inited-social ]]"):
                warning("Social network seems uninitialized. Populating with data...")
                url = self.frontend_url.split(":")
                assert len(url) == 2
                social_graph = "socfb-Reed98"
                # doesnt fail if run twice, so we could remove the -f /inited-social check
                loadgen.exec(f"cd {PROJECT_ROOT}/subprojects/deathstarbench/socialNetwork; nix develop {PROJECT_ROOT}# --command python3 scripts/init_social_graph.py --graph {social_graph} --limit {connections} --ip {url[0]} --port {url[1]} 2>&1 | tee /tmp/init-social.log")
                frontend_guest.exec("touch /inited-social")
        if self.app == "mediaMicroservices":
            frontend_guest = guests[self.vm_number_of(self.frontend)]
            # if True or not frontend_guest.test("[[ -f /inited-media ]]"):
            #     warning("Media microservices seem uninitialized. Populating with data...")
            # tests fails if this is not run after a VM/docker-compose restart
            url = self.frontend_url.split("/")[0]
            prefix = f"{PROJECT_ROOT}/subprojects/deathstarbench/mediaMicroservices"
            loadgen.exec(f"cd {prefix}; nix develop {PROJECT_ROOT}# --command python3 scripts/write_movie_info.py -c {prefix}/datasets/tmdb/casts.json -m {prefix}/datasets/tmdb/movies.json --limit {connections} --server_address http://{url} 2>&1 | tee /tmp/init-media.log")
            frontend_guest.exec("./scripts/register_users.sh")
            frontend_guest.exec("./scripts/register_movies.sh")
            frontend_guest.exec("touch /inited-media")

        # run actual test

        info(f"Running wrk2 {test.repetitions} times")

        remote_output_file = "/tmp/output.log"
        for repetition in range(test.repetitions):
            local_output_file = test.output_filepath(repetition)
            loadgen.exec(f"sudo rm {remote_output_file} || true")
            LoadGen.stop_wrk2(loadgen)
            workdir = f"{loadgen.moonprogs_dir}/../../subprojects/deathstarbench/{self.app}"
            LoadGen.start_wrk2(loadgen, self.frontend_url, self.script, duration=DURATION_S, outfile=remote_output_file, workdir=workdir)
            time.sleep(DURATION_S + 1)
            try:
                loadgen.wait_for_success(f'[[ $(tail -n 1 {remote_output_file}) = *"AUTOTEST_DONE"* ]]')
            except TimeoutError:
                error('Waiting for output file to appear timed out')
            loadgen.copy_from(remote_output_file, local_output_file)

            LoadGen.stop_wrk2(loadgen)


        # stop docker-compose

        def foreach_parallel(i, guest): # pyright: ignore[reportGeneralTypeIssues]
            self.docker_compose_cleanup(guest)
        end_foreach(guests, foreach_parallel)


def main(measurement: Measurement, plan_only: bool = False) -> None:
    # general measure init
    # host, loadgen = measurement.hosts()
    global DURATION_S

    interfaces = [
        Interface.VMUX_EMU,
        # Interface.VMUX_DPDK, # multi-vm broken right now
        Interface.BRIDGE_E1000,
        Interface.BRIDGE,
        Interface.BRIDGE_VHOST,
        # Interface.VMUX_DPDK_E810,
        ]
    rpsList = [ 10, 100, 200, 300, 400, 500, 600 ]
    apps = [ "hotelReservation", "socialNetwork", "mediaMicroservices" ]
    repetitions = 4
    DURATION_S = 61 if not G.BRIEF else 11
    if G.BRIEF:
        interfaces = [ Interface.BRIDGE_E1000 ]
        # interfaces = [ Interface.VMUX_EMU ]
        rpsList = [ 10 ]
        # apps = [ "hotelReservation" ]
        # apps = [ "socialNetwork" ]
        apps = [ "mediaMicroservices" ]
        repetitions = 1

    # plan the measurement
    # moonprogs_dir = f"{measurement.config['guest']['moonprogs_dir']}"
    moonprogs_dir = "/tmp/none"
    # deathstarbenches = {app: DeathStarBench(app, moonprogs_dir) for app in apps}
    deathstarbenches = {app: len(app) for app in apps}
    tests = []
    time_needed_s = 0
    for app in apps:
        bench_app = deathstarbenches[app]
        test_matrix = dict(
            interface=[ interface.value for interface in interfaces],
            rps=rpsList,
            app=[ app ],
            repetitions=[ repetitions ],
            num_vms=[ bench_app ]
            )
        info(f"DeathStarBench execution plan {app}:")
        time_needed_s += DeathStarBenchTest.estimate_time(test_matrix, args_reboot = ["app", "interface", "num_vms"])
        tests += DeathStarBenchTest.list_tests(test_matrix)

    with Bench(tests = tests, args_reboot = ["app", "interface", "num_vms"], brief = G.BRIEF) as (bench, bench_tests):
        for app, app_tests in bench.iterator(bench_tests, "app"):
            print(f"- Doing app {app}")
            for interface, interface_tests in bench.iterator(app_tests, "interface"):
                print(f"-- Doing interface {interface}")
                for test in interface_tests:
                    print(f"--- Doing test {test}")
                    time.sleep(.1)
                    bench.done(test)

    exit(0)
    if plan_only:
        return

    for app in apps:

        deathstar = deathstarbenches[app]
        num_vms = len(deathstar.docker_compose)

        for interface in interfaces:

            # skip VM boots if possible
            test_matrix = dict(
                interface=[ interface.value ],
                rps=rpsList,
                app=[ app ],
                repetitions=[ repetitions ],
                num_vms=[ num_vms ]
                )
            if not DeathStarBenchTest.any_needed(test_matrix):
                warning(f"Skipping app {app} with {interface.value}: All measurements done already.")
                continue

            # boot VMs
            with measurement.virtual_machines(interface, num=num_vms) as guests:
                # loadgen: set up interfaces and networking

                debug('Binding loadgen interface')
                loadgen.modprobe_test_iface_drivers()
                loadgen.release_test_iface() # bind linux driver
                try:
                    loadgen.delete_nic_ip_addresses(loadgen.test_iface)
                except Exception:
                    pass
                loadgen.setup_test_iface_ip_net()

                def foreach_parallel(i, guest): # pyright: ignore[reportGeneralTypeIssues]
                    guest.modprobe_test_iface_drivers(interface=interface)
                    guest.setup_test_iface_ip_net()
                end_foreach(guests, foreach_parallel)

                for rps in rpsList:
                    # the actual test
                    test = DeathStarBenchTest(
                            repetitions=repetitions,
                            app=app,
                            rps=rps,
                            interface=interface.value,
                            num_vms=num_vms
                            )
                    if test.needed():
                        info(f"running {test}")
                        deathstar.run(host, loadgen, guests, test)
                    else:
                        info(f"skipping {test}")

    DeathStarBench.find_errors(G.OUT_DIR)

if __name__ == "__main__":
    # measurement = Measurement()
    measurement = None
    main(measurement)
