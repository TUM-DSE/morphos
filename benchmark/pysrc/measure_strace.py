import re
from tqdm import tqdm
import pandas as pd
import glob

# Regular expression to parse the strace output
strace_pattern = re.compile(r'(\w+)\((.*?)\)\s+=\s+(-?\d+|0x[0-9a-fA-F]+)(?:\s+\((.*?)\))?')

# Parse the strace output
def parse_strace_output(lines):
    parsed_data = []
    for line_ in tqdm(lines):
        starts_with_pid = line_.split(' ')[0].isdigit() if (len(line_.split(' ')) > 0) else False
        if starts_with_pid:
            line = " ".join(line_.split(' ')[1:])
        else:
            line = line_
        match = strace_pattern.match(line)
        if match:
            syscall = match.group(1)
            args = match.group(2)
            result = match.group(3)
            error = match.group(4) if match.group(4) else None
            parsed_data.append({
                'syscall': syscall,
                'args': args,
                'result': result,
                'error': error
            })
    return parsed_data

def log(msg: str):
    print(msg, flush=True)

def collect_histograms(files, ignored_syscalls=[]):
    syscall_histograms = dict()
    for category, file_glob in files.items():
        dfs = []
        for file in glob.glob(file_glob):
            with open(file, 'r') as f:
                log(f"reading {file}")
                lines = f.readlines()

            # Parse the provided strace output
            log(f"parsing {file}")
            parsed_strace = parse_strace_output(lines)
            dfs += [ pd.DataFrame(parsed_strace) ]
        df = pd.concat(dfs)
        df = df[~df.syscall.isin(ignored_syscalls)]
        syscall_counts = df['syscall'].value_counts()
        syscall_histograms[category] = syscall_counts
    return syscall_histograms

def merge_histograms(syscall_histograms, categoryA, categoryB, how='outer'):
    full_join = pd.merge(syscall_histograms[categoryA], syscall_histograms[categoryB], on='syscall', how=how, suffixes=(f"_{categoryA}", f"_{categoryB}"))
    outer_join = full_join[full_join.isna().any(axis=1)]
    return full_join, outer_join

# file paths support globbing
files = {
        "allnet": "./strace-allnet",
        "nonet": "./strace-nonet",
        "vmux": "./strace-vmux",
        # "literal-vmux": "./strace-literal-vmux" # data missing
        "literal-vmux": "./strace-runtime/strace-literal-vmux.*"
         }
ignored_syscalls = [
        "restart_syscall" # not a syscall we can block
        ]
categories = files.keys()
syscall_histograms = collect_histograms(files, ignored_syscalls=ignored_syscalls)

log("")
log("vmux syscalls")
log(syscall_histograms['vmux'])
log(f"Qemu-vmux uses {len(syscall_histograms['vmux'])} syscalls")

full, outer = merge_histograms(syscall_histograms, "allnet", "vmux", how="left")
log(outer.to_string())
vmux_can_prohibit = outer

full, outer = merge_histograms(syscall_histograms, "allnet", "vmux", how="right")
log(outer.to_string())
vmux_needs_additionally = outer

log(f"vmux can block {len(vmux_can_prohibit)} syscalls, but needs {len(vmux_needs_additionally)} additional syscalls")

new = len(syscall_histograms['vmux'])
old = len(syscall_histograms['allnet'])
decrease = (new - old) / old
log(f"change of {decrease*100}%")

log("crucially, it can restrict syscalls to open network connections from 'listen(for any ip)' to connect(to vmux socket)'")

# =======================
# Compare literal-vmux vs qemu

log("")
log(f"Qemu uses {len(syscall_histograms['allnet'])} syscalls")
log(f"Qemu-vmux uses {len(syscall_histograms['vmux'])} syscalls")
log(f"vMux uses {len(syscall_histograms['literal-vmux'])} syscalls")

log("")

log(f"Qemu uses the foolowing syscalls: {' '.join(syscall_histograms['allnet'].keys())}")
log(f"vMux uses the foolowing syscalls: {' '.join(syscall_histograms['literal-vmux'].keys())}")

vmux_more_syscalls_than_qemu = len(syscall_histograms['literal-vmux']) - len(syscall_histograms['vmux'])
full, outer = merge_histograms(syscall_histograms, 'literal-vmux', 'vmux', how='inner')
vmux_qemu_share_syscalls = len(full)
full, outer = merge_histograms(syscall_histograms, 'literal-vmux', 'vmux', how='left')
vmux_only_syscalls = len(outer)
full, outer = merge_histograms(syscall_histograms, 'literal-vmux', 'vmux', how='right')
qemu_only_syscalls = len(outer)
log(f"vMux, although not a hypervisor, is a complex application - particularly because of the feature-richness of DPDK. ")
log(f"vMux uses {vmux_more_syscalls_than_qemu} syscalls more than qemu. ")
log(f"However, by splitting the networking components and Qemu into seperate processes, we can employ several isolation mechanisms:")
log(f"(1) We run vMux and Qemu as separate, non-root users. vMux doesnt gain access to KVM resources, Qemu doesn't gain VFIO resources.")
log(f"(2) We use linux namespaces to permit network device access only for vMux and file system access only for Qemu.")
log(f"This is a powerful restriction because, with vMux, Qemu needs no network access anymore, but maintains lots of persistent state e.g. for disk and EFI emulation whereas vMux maintains none.") # TODO writes to /var/run/dpdk!
# literal-vmux opens libraries and /sys and /proc and /dev/hugepages* and /dev/vfio and /etc/localtime and /var/run/dpdk and /lib/firmware
log(f"(3) We reduce the kernels attack surface, by starting vMux and Qemu with our syscall jailer.")
log(f"vMux and qemu share {vmux_qemu_share_syscalls} syscalls, vmux uses {vmux_only_syscalls} syscalls that qemu doesnt use, qemu uses {qemu_only_syscalls} that vmux doesnt use.")

# log(f"The main behavioural difference between vMux and Qemu is, that Qemu reads and writes lots of state files e.g. to emulate disk and EFI. ")
# log("vMux barely uses the filesystem.")
# log("We can use namespaces to restrict Qemus network access, while also restricting vMux's filesystem access.")


log("")
log("WARNING: note that you need to collect the traces (just vm-strace*) while doing a network request at least once")


# ==============
# start counting only after vmux initialization?
# - vmux waits for qemu after printing "Waiting for qemu to attach..."
# - vmux has initialized the device and dpdk before the first "unknown opcode=" or "nvm reat at"

breakpoint()
