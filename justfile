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

vm EXTRA_CMDLINE="" :
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
  conf.read("{{proot}}/benchmark/conf/autotest_localhost.cfg")
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
  conf.read("{{proot}}/benchmark/conf/autotest_localhost.cfg")
  import os
  sudo = ""
  if conf["host"]["ssh_as_root"]:
    sudo = "sudo "
  cmd = f"{sudo}ssh -F {conf['host']['ssh_config']} {conf['guest']['fqdn']} {{ARGS}}"
  print(f"$ {cmd}")
  os.system(cmd)

benchmark:
  python3 benchmark/pysrc/measure_throughput.py -c benchmark/conf/autotest_localhost.cfg -b -vvv

build-dependencies:
  mkdir -p {{proot}}/nix/builds
  nix build .#click -o {{proot}}/nix/builds/click
  nix build .#linux-pktgen -o {{proot}}/nix/builds/linux-pktgen

build-click-og:
  nix develop --unpack .#click
  mv source libs/click-og
  cd libs/click-og && nix develop .#click --command bash -c 'eval "$postPatch"'
  cd libs/click-og && nix develop .#click --command bash -c './configure'
  cd libs/click-og && nix develop .#click --command bash -c 'make -j$(nproc)'
