proot := justfile_directory()
qemu_ssh_port := "2222"
user := `whoami`

default:
    @just --choose

help:
    just --list

ssh COMMAND="":
    ssh \
    -i {{proot}}/nix/ssh_key \
    -o StrictHostKeyChecking=no \
    -o UserKnownHostsFile=/dev/null \
    -o IdentityAgent=/dev/null \
    -F /dev/null \
    -p {{qemu_ssh_port}} \
    root@localhost -- "{{COMMAND}}"

vm-linux EXTRA_CMDLINE="" :
    sudo qemu-system-x86_64 \
        -cpu host \
        -smp 4 \
        -enable-kvm \
        -m 16G \
        -machine q35,accel=kvm,kernel-irqchip=split \
        -device intel-iommu,intremap=on,device-iotlb=on,caching-mode=on \
        -device virtio-serial \
        -fsdev local,id=home,path={{proot}},security_model=none \
        -device virtio-9p-pci,fsdev=home,mount_tag=home,disable-modern=on,disable-legacy=off \
        -fsdev local,id=nixstore,path=/nix/store,security_model=none \
        -device virtio-9p-pci,fsdev=nixstore,mount_tag=nixstore,disable-modern=on,disable-legacy=off \
        -drive file={{proot}}/VMs/guest-image.qcow2 \
        -net nic,netdev=user.0,model=virtio \
        -netdev user,id=user.0,hostfwd=tcp:127.0.0.1:{{qemu_ssh_port}}-:22 \
        -netdev bridge,id=en0,br=clicknet \
        -device virtio-net-pci,netdev=en0 \
        -nographic


#-device vfio-pci,host={{PASSTHROUGH}} \
#PASSTHROUGH=`yq -r '.devices[] | select(.name=="ethDut") | ."pci"' hosts/$(hostname).yaml`

vm-image-init:
    #!/usr/bin/env bash
    set -x
    set -e
    echo "Initializing disk for the VM"
    mkdir -p {{proot}}/VMs

    # build images fast
    overwrite() {
        install -D -m644 {{proot}}/VMs/ro/nixos.qcow2 {{proot}}/VMs/$1.qcow2
        qemu-img resize {{proot}}/VMs/$1.qcow2 +8g
    }

    nix build .#guest-image --out-link {{proot}}/VMs/ro
    overwrite guest-image

# use autotest tmux sessions: `just autotest-tmux ls`
autotest-tmux *ARGS:
  #!/usr/bin/env python3
  from configparser import ConfigParser, ExtendedInterpolation
  import importlib.util
  spec = importlib.util.spec_from_file_location("default_parser", "benchmark/pysrc/conf.py")
  default_parser = importlib.util.module_from_spec(spec)
  spec.loader.exec_module(default_parser)
  conf = default_parser.default_config_parser()
  conf.read("{{proot}}/benchmark/conf/uk_localhost.cfg")
  import os
  os.system(f"tmux -L {conf['common']['tmux_socket']} {{ARGS}}")

# connect to the autotest guest
autotest-ssh *ARGS:
  #!/usr/bin/env python3
  from configparser import ConfigParser, ExtendedInterpolation
  import importlib.util
  spec = importlib.util.spec_from_file_location("default_parser", "benchmark/pysrc/conf.py")
  default_parser = importlib.util.module_from_spec(spec)
  spec.loader.exec_module(default_parser)
  conf = default_parser.default_config_parser()
  conf.read("{{proot}}/benchmark/conf/uk_localhost.cfg")
  import os
  sudo = ""
  if conf["host"]["ssh_as_root"]:
    sudo = "sudo "
  cmd = f"{sudo}ssh -F {conf['host']['ssh_config']} {conf['guest']['fqdn']} {{ARGS}}"
  print(f"$ {cmd}")
  os.system(cmd)

benchmark:
  python3 benchmark/pysrc/measure_throughput.py -c benchmark/conf/uk_localhost.cfg -b -vvv

build-dependencies:
  mkdir -p {{proot}}/nix/builds
  nix build .#linux-pktgen -o {{proot}}/nix/builds/linux-pktgen
  nix build --inputs-from . nixpkgs#qemu -o {{proot}}/nix/builds/qemu
  nix build .#vpp2 -o {{proot}}/nix/builds/vpp
  nix build .#click -o {{proot}}/nix/builds/click
  nix build -o {{proot}}/nix/builds/xdp github:vmuxio/vmuxio#xdp-reflector

build-click-og:
  nix develop --unpack .#click
  mv source libs/click-og
  cd libs/click-og && nix develop .#click --command bash -c 'eval "$postPatch"'
  cd libs/click-og && nix develop .#click --command bash -c './configure'
  cd libs/click-og && nix develop .#click --command bash -c 'make -j$(nproc)'


downloadLibs:
    @nix develop .#unikraft --command bash -c 'sourceRoot=$(pwd); eval "$postUnpack"'

kill:
        sudo pkill -f "clicknet"
        sudo pkill -f "controlnet"

throughput-cpio:
    rm -r /tmp/ukcpio-$(USER) || true
    mkdir -p /tmp/ukcpio-$(USER)
    cp ./throughput.click /tmp/ukcpio-$(USER)/config.click
    ./libs/unikraft/support/scripts/mkcpio ./throughput.cpio /tmp/ukcpio-$(USER)

nat-cpio:
    rm -r /tmp/ukcpio-{{user}} || true
    mkdir -p /tmp/ukcpio-{{user}}
    cp ./benchmark/configurations/thomer-nat.click /tmp/ukcpio-{{user}}/config.click
    ./libs/unikraft/support/scripts/mkcpio ./throughput.cpio /tmp/ukcpio-{{user}}

stringmatcher-cpio:
    rm -r /tmp/ukcpio-{{user}} || true
    mkdir -p /tmp/ukcpio-{{user}}
    cp ./benchmark/configurations/stringmatcher.click /tmp/ukcpio-{{user}}/config.click
    cp ./benchmark/bpfilters/stringmatcher /tmp/ukcpio-{{user}}/stringmatcher
    cp ./benchmark/bpfilters/stringmatcher.sig /tmp/ukcpio-{{user}}/stringmatcher.sig
    ./libs/unikraft/support/scripts/mkcpio ./throughput.cpio /tmp/ukcpio-{{user}}

natebpf-cpio:
    rm -r /tmp/ukcpio-{{user}} || true
    mkdir -p /tmp/ukcpio-{{user}}
    # cp ./benchmark/configurations/thomer-nat-ebpf.click /tmp/ukcpio-{{user}}/config.click
    cp ./benchmark/configurations/thomer-nat.click /tmp/ukcpio-{{user}}/config.click
    # cp ./benchmark/configurations/test.click /tmp/ukcpio-{{user}}/config.click
    # cp ./benchmark/configurations/test2.click /tmp/ukcpio-{{user}}/config.click
    # cp ./benchmark/configurations/stringmatcher.click /tmp/ukcpio-{{user}}/config.click
    cp ./benchmark/bpfilters/round-robin /tmp/ukcpio-{{user}}/round-robin
    cp ./benchmark/bpfilters/round-robin.sig /tmp/ukcpio-{{user}}/round-robin.sig
    cp ./benchmark/bpfilters/nat /tmp/ukcpio-{{user}}/nat
    cp ./benchmark/bpfilters/nat.sig /tmp/ukcpio-{{user}}/nat.sig
    ./libs/unikraft/support/scripts/mkcpio ./throughput.cpio /tmp/ukcpio-{{user}}

vm: natebpf-cpio
    sudo taskset -c 3,4 qemu-system-x86_64 \
        -accel kvm -cpu max \
        -m 1024M -object memory-backend-file,id=mem,size=1024M,mem-path=/dev/hugepages,share=on \
        -mem-prealloc -numa node,memdev=mem \
        -netdev bridge,id=en0,br=clicknet \
        -device virtio-net-pci,netdev=en0 \
        -append " vfs.fstab=[\"initrd0:/:extract::ramfs=1:\"] --" \
        -kernel ./.unikraft/build/click_qemu-x86_64 \
        -initrd ./throughput.cpio \
        -nographic


vm-vhost: throughput-cpio
    sudo taskset -c 3,4 qemu-system-x86_64 \
        -accel kvm -cpu max \
        -m 1024M -object memory-backend-file,id=mem,size=1024M,mem-path=/dev/hugepages,share=on \
        -mem-prealloc -numa node,memdev=mem \
        -chardev socket,id=char1,path=/tmp/vhost-user-okelmann-0,server \
        -netdev type=vhost-user,id=hostnet1,chardev=char1  \
        -device virtio-net-pci,netdev=hostnet1,id=net1,mac=52:54:00:00:00:14 \
        -append " vfs.fstab=[\"initrd0:/:extract::ramfs=1:\"] --" \
        -kernel ./.unikraft/build/click_qemu-x86_64 \
        -initrd ./throughput.cpio \
        -nographic


vhost-user:
    sudo ./result-examples/bin/dpdk-vhost -l 5-8 -n 4 --socket-mem 1024 -- --socket-file /tmp/vhost-user0 --client -p 1 --stats 1


vpp-notes:
    # sudo vpp -c ./vpp.conf
    # sudo vppctl -s /tmp/vpp-cli
    # show log
    # show interface
    # create vhost-user socket /tmp/vhost-user0
    # set int state VirtualEthernet0/0/0 up
    # set interface l2 xconnect GigabitEthernet0/8/0.300 GigabitEthernet0/9/0.300


perf-kvm-top:
    sudo perf kvm --guestkallsyms=./.unikraft/build/click_qemu-x86_64.sym --guestvmlinux=.unikraft/build/click_qemu-x86_64.dbg top -p $(pgrep qemu)

perf-kvm-record:
    sudo perf kvm --guestkallsyms=./.unikraft/build/click_qemu-x86_64.sym --guestvmlinux=.unikraft/build/click_qemu-x86_64.dbg record -g -p $(pgrep qemu)
    sudo perf kvm --guestkallsyms=./.unikraft/build/click_qemu-x86_64.sym --guestvmlinux=.unikraft/build/click_qemu-x86_64.dbg report

perf-qemu-record:
    sudo perf record -g -p $(pgrep qemu)
    sudo perf script > perf.trace

UBUNTU_PATH := "~/.vagrant.d/boxes/ubuntu-VAGRANTSLASH-jammy64/20241002.0.0/virtualbox/ubuntu-jammy-22.04-cloudimg.vmdk"
ALPINE_PATH := "~/.vagrant.d/boxes/generic-VAGRANTSLASH-alpine319/4.3.12/virtualbox/generic-alpine319-virtualbox-x64-disk001.vmdk"

imagesizes: natebpf-cpio
    # downloading images
    [ -e {{ALPINE_PATH}} ] || nix run --inputs-from ./ nixpkgs#vagrant -- box add generic/alpine319 --provider virtualbox --box-version 4.3.12
    [ -e {{UBUNTU_PATH}} ] || nix run --inputs-from ./ nixpkgs#vagrant -- box add ubuntu/jammy64 --provider virtualbox --box-version 20241002.0.0
    # click unikraft nat ebpf
    ls -l ./.unikraft/build/click_qemu-x86_64
    ls -l ./throughput.cpio
    # click linux nat ebpf
    # we should also count non-trivial click dependencies: dpdk, libjannson
    ls -l ./nix/builds/click/bin/click
    ls -l ./benchmark/configurations/thomer-nat-ebpf.click
    ls -l ./benchmark/bpfilters/nat
    ls -l ./benchmark/bpfilters/nat.sig
    ls -l {{ALPINE_PATH}}
    ls -l {{UBUNTU_PATH}}


