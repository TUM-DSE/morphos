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

# build unikraft images incrementally
build-vm-images: vm-image-init
  #!/usr/bin/env bash
  rm -rf {{proot}}/.unikraft/build/libclick/origin/click-a5384835a6cac10f8d44da4eeea8eaa8f8e6a0c2/elements/unikraft || true
  mkdir -p {{proot}}/.unikraft/build/libclick/origin/click-a5384835a6cac10f8d44da4eeea8eaa8f8e6a0c2/elements/unikraft || true
  cp -r {{proot}}/libs/click/unikraft {{proot}}/.unikraft/build/libclick/origin/click-a5384835a6cac10f8d44da4eeea8eaa8f8e6a0c2/elements
  rm -f {{proot}}/.unikraft/build/libclick/origin/click-a5384835a6cac10f8d44da4eeea8eaa8f8e6a0c2/include/click/packet.hh || true
  rm -f {{proot}}/.unikraft/build/libclick/origin/click-a5384835a6cac10f8d44da4eeea8eaa8f8e6a0c2/lib/packet.cc || true
  cp {{proot}}/libs/click/packet.hh {{proot}}/.unikraft/build/libclick/origin/click-a5384835a6cac10f8d44da4eeea8eaa8f8e6a0c2/include/click/packet.hh
  cp {{proot}}/libs/click/packet.cc {{proot}}/.unikraft/build/libclick/origin/click-a5384835a6cac10f8d44da4eeea8eaa8f8e6a0c2/lib/packet.cc
  rm .config.click_qemu-x86_64
  kraft build -K Kraftfile
  cp .unikraft/build/click_qemu-x86_64 VMs/unikraft_mpk
  rm .config.click_qemu-x86_64
  kraft build -K Kraftfile_nompk
  cp .unikraft/build/click_qemu-x86_64 VMs/unikraft
  rm .config.vanilla_qemu-x86_64
  kraft build -K Kraftfile_vanilla
  cp .unikraft/build/click_qemu-x86_64 VMs/unikraft_vanilla

# build unikraft images reproducably
build-morphos:
  mkdir -p {{proot}}/nix/builds
  mkdir -p {{proot}}/VMs
  rm -f VMs/unikraft_mpk VMs/unikraft VMs/unikraft_nopaging VMs/unikraft_vanilla || true
  nix build .#morphos -o {{proot}}/nix/builds/morphos
  cp {{proot}}/nix/builds/morphos/click_qemu-x86_64 VMs/unikraft
  nix build .#morphos-mpk -o {{proot}}/nix/builds/morphos-mpk
  cp {{proot}}/nix/builds/morphos-mpk/click_qemu-x86_64 VMs/unikraft_mpk
  nix build .#morphos-nopaging -o {{proot}}/nix/builds/morphos-nopaging
  cp {{proot}}/nix/builds/morphos-nopaging/click_qemu-x86_64 VMs/unikraft_nopaging
  nix build .#unikraft-vanilla -o {{proot}}/nix/builds/unikraft-vanilla
  cp {{proot}}/nix/builds/unikraft-vanilla/vanilla_qemu-x86_64 VMs/unikraft_vanilla

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
  nix build --inputs-from . nixpkgs#time -o {{proot}}/nix/builds/time
  nix build .#moongen-lachnit -o {{proot}}/nix/builds/moongen-lachnit

build-click-og:
  nix develop --unpack .#click
  mv source libs/click-og
  cd libs/click-og && nix develop .#click --command bash -c 'eval "$postPatch"'
  cd libs/click-og && nix develop .#click --command bash -c './configure'
  cd libs/click-og && nix develop .#click --command bash -c 'make -j$(nproc)'


downloadLibs:
    @nix develop .#unikraft --command bash -c 'sourceRoot=$(pwd); export SKIP_UNPACK_UNIKRAFT=1; eval "$postUnpack"'

kill:
        sudo pkill -f "clicknet"
        sudo pkill -f "controlnet"

throughput-cpio:
    rm -r /tmp/ukcpio-$(USER) || true
    mkdir -p /tmp/ukcpio-$(USER)
    cp ./throughput.click /tmp/ukcpio-$(USER)/config.click
    ./libs/unikraft/support/scripts/mkcpio ./throughput.cpio /tmp/ukcpio-$(USER)

nat-cpio:
    #!/usr/bin/env bash
    rm -r /tmp/ukcpio-{{user}} || true
    mkdir -p /tmp/ukcpio-{{user}}
    cp ./benchmark/configurations/thomer-nat.click /tmp/ukcpio-{{user}}/config.click
    ./libs/unikraft/support/scripts/mkcpio ./nat.cpio /tmp/ukcpio-{{user}} > /dev/null 2> /dev/null

stringmatcher-cpio:
    rm -r /tmp/ukcpio-{{user}} || true
    mkdir -p /tmp/ukcpio-{{user}}
    cp ./benchmark/configurations/stringmatcher.click /tmp/ukcpio-{{user}}/config.click
    cp ./benchmark/bpfilters/stringmatcher /tmp/ukcpio-{{user}}/stringmatcher
    cp ./benchmark/bpfilters/stringmatcher.sig /tmp/ukcpio-{{user}}/stringmatcher.sig
    ./libs/unikraft/support/scripts/mkcpio ./throughput.cpio /tmp/ukcpio-{{user}}

natebpf-cpio:
    #!/usr/bin/env bash
    rm -r /tmp/ukcpio-{{user}} || true
    mkdir -p /tmp/ukcpio-{{user}}
    cp ./benchmark/configurations/thomer-nat-ebpf.click /tmp/ukcpio-{{user}}/config.click
    # cp ./benchmark/configurations/thomer-nat.click /tmp/ukcpio-{{user}}/config.click
    # cp ./benchmark/configurations/test.click /tmp/ukcpio-{{user}}/config.click
    # cp ./benchmark/configurations/test2.click /tmp/ukcpio-{{user}}/config.click
    # cp ./benchmark/configurations/stringmatcher.click /tmp/ukcpio-{{user}}/config.click
    #cp ./benchmark/bpfilters/round-robin /tmp/ukcpio-{{user}}/round-robin
    #cp ./benchmark/bpfilters/round-robin.sig /tmp/ukcpio-{{user}}/round-robin.sig
    cp ./benchmark/bpfilters/nat /tmp/ukcpio-{{user}}/nat
    cp ./benchmark/bpfilters/nat.sig /tmp/ukcpio-{{user}}/nat.sig
    #cp ./benchmark/bpfilters/firewall-2 /tmp/ukcpio-{{user}}/firewall-2
    #cp ./benchmark/bpfilters/firewall-2.sig /tmp/ukcpio-{{user}}/firewall-2.sig
    #cp ./benchmark/bpfilters/firewall-10000 /tmp/ukcpio-{{user}}/firewall-10000
    #cp ./benchmark/bpfilters/firewall-10000.sig /tmp/ukcpio-{{user}}/firewall-10000.sig
    ./libs/unikraft/support/scripts/mkcpio ./natebpf.cpio /tmp/ukcpio-{{user}} > /dev/null 2> /dev/null

vm: natebpf-cpio
    sudo taskset -c 3,4 qemu-system-x86_64 \
        -accel kvm -cpu max \
        -m 12G -object memory-backend-file,id=mem,size=12G,mem-path=/dev/hugepages,share=on \
        -mem-prealloc -numa node,memdev=mem \
        -netdev bridge,id=en0,br=clicknet \
        -device virtio-net-pci,netdev=en0 \
        -append " vfs.fstab=[\"initrd0:/:extract::ramfs=1:\"] --" \
        -initrd natebpf.cpio \
        -kernel VMs/unikraft \
        -nographic

vm_nompk: natebpf-cpio
    sudo taskset -c 3,4 qemu-system-x86_64 \
        -accel kvm -cpu max \
        -m 12G -object memory-backend-file,id=mem,size=12G,mem-path=/dev/hugepages,share=on \
        -mem-prealloc -numa node,memdev=mem \
        -netdev bridge,id=en0,br=clicknet \
        -device virtio-net-pci,netdev=en0 \
        -append " vfs.fstab=[\"initrd0:/:extract::ramfs=1:\"] --" \
        -initrd natebpf.cpio \
        -kernel VMs/unikraft_nompk \
        -nographic

#-kernel ./.unikraft/build/click_qemu-x86_64 \

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
    # sudo vpp -c ./benchmark/configurations/vpp.conf
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

qemu-startup:
    BPFTRACE_MAX_STRLEN=123 sudo -E bpftrace -e " \
    tracepoint:kvm:kvm_entry / @a[pid] == 0 / { printf(\"qemu kvm entry ns %lld\n\", nsecs()); @a[pid] = 1; } \
    tracepoint:kvm:kvm_pio / args.port == 0xf4 / { printf(\"qemu kvm port %d ns %lld\n\", args->val, nsecs()); } \
    tracepoint:syscalls:sys_enter_execve* \
    / str(args.filename) == \"$(which qemu-system-x86_64)\" / \
    { printf(\"qemu start ns %lld\n\", nsecs()); printf(\"filename %s\n\", str(args.filename))}" \


UBUNTU_PATH := "~/.vagrant.d/boxes/ubuntu-VAGRANTSLASH-jammy64/20241002.0.0/virtualbox/ubuntu-jammy-22.04-cloudimg.vmdk"
ALPINE_PATH := "~/.vagrant.d/boxes/generic-VAGRANTSLASH-alpine319/4.3.12/virtualbox/generic-alpine319-virtualbox-x64-disk001.vmdk"

imagesizes: nat-cpio natebpf-cpio build-morphos
    #!/usr/bin/env bash
    [ -e {{ALPINE_PATH}} ] || nix run --inputs-from ./ nixpkgs#vagrant -- box add generic/alpine319 --provider virtualbox --box-version 4.3.12 > /dev/null 2> /dev/null
    [ -e {{UBUNTU_PATH}} ] || nix run --inputs-from ./ nixpkgs#vagrant -- box add ubuntu/jammy64 --provider virtualbox --box-version 20241002.0.0
    uk_click=$(ls -l VMs/unikraft | awk '{print $5;}')
    uk_vanilla=$(ls -l VMs/unikraft_vanilla | awk '{print $5;}')
    uk_cpio=$(ls -l ./nat.cpio | awk '{print $5;}')
    uk_ebpfcpio=$(ls -l ./natebpf.cpio | awk '{print $5;}')
    # we should also count non-trivial click dependencies: dpdk, libjannson
    click=$(ls -l ./nix/builds/click/bin/click | awk '{print $5;}')
    conf_natbpf=$(ls -l ./benchmark/configurations/thomer-nat-ebpf.click | awk '{print $5;}')
    natbpf=$(ls -l ./benchmark/bpfilters/nat | awk '{print $5;}')
    sig=$(ls -l ./benchmark/bpfilters/nat.sig | awk '{print $5;}')
    alpine=$(ls -l {{ALPINE_PATH}} | awk '{print $5;}')
    ubuntu=$(ls -l {{UBUNTU_PATH}} | awk '{print $5;}')
    echo "system,size"
    echo "Unikraft,"$(expr $uk_vanilla \+ $uk_cpio )
    echo "MorphOS,"$(expr $uk_click \+ $uk_ebpfcpio )
    echo "Alpine,"$(expr $alpine \+ $click \+ $conf_natbpf \+ $natbpf \+ $sig )
    echo "Ubuntu,"$(expr $ubuntu \+ $click \+ $conf_natbpf \+ $natbpf \+ $sig )

nat_buildtime OUTFILE="bpfbuild.csv":
    #!/usr/bin/env bash
    set -x
    tmp=$(mktemp)
    cd ebpf; cargo clean; cd ..
    make -C ebpf dns-filter VERIFY=1 RECORD=1 > /dev/null 2> /dev/null
    echo "system,contributor,restart_s" > {{OUTFILE}}
    sudo bpftrace -q -e 'tracepoint:syscalls:sys_enter_execve /str(args->filename) == "'$HOME'/.cargo/bin/bpf-linker"/ { @start = nsecs; @p = pid; } tracepoint:syscalls:sys_enter_exit* / pid == @p / { printf("Out-of-band,Link,%d\n", (nsecs - @start) / 1000000); }' >> {{OUTFILE}} &
    make -C ebpf nat VERIFY=1 RECORD=1 > $tmp 2> $tmp
    cat $tmp | grep -E "Verification took|real|Building" | sed -e 's/\x1b\[[0-9;]*m//g' | awk '/^real/{split($2,t,/[ms]/); printf "Out-of-band,Compile,%d\n", t[2]*1000} /^Verification/{printf "Out-of-band,Verify,%.1f\n", $3/1e6}' >> {{OUTFILE}}
    sudo kill -9 $(pidof bpftrace)

build-verifier:
    make -C verifier build -j

build-ebpf:
    make -C ebpf all VERIFY=1
    make -C ebpf sync
    make -C ebpf sync-firewalls VERIFY=1

archive:
    git clone https://github.com/TUM-DSE/morphos.git --recursive /tmp/morphos
    cd /tmp/morphos; just downloadLibs
    tar -czf morphos.tar.gz /tmp/morphos
    rm -rf /tmp/morphos
