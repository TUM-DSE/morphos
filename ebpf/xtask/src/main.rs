mod build_ebpf;

use std::process::exit;

use clap::Parser;

#[derive(Debug, Parser)]
pub struct Options {
    #[clap(subcommand)]
    command: Command,
}

#[derive(Debug, Parser)]
enum Command {
    BuildPassEbpf(build_ebpf::Options),
    BuildDropEbpf(build_ebpf::Options),
    BuildTargetPortEbpf(build_ebpf::Options),
    BuildRateLimiterEbpf(build_ebpf::Options),
    BuildDnsFilterEbpf(build_ebpf::Options),
    BuildUdpTcpClassifierEbpf(build_ebpf::Options),
    BuildEtherMirrorEbpf(build_ebpf::Options),
    BuildStripEtherVlanHeaderEbpf(build_ebpf::Options),
}

fn main() {
    let opts = Options::parse();

    use Command::*;
    let ret = match opts.command {
        BuildPassEbpf(opts) => build_ebpf::build_ebpf("pass-ebpf", opts),
        BuildDropEbpf(opts) => build_ebpf::build_ebpf("drop-ebpf", opts),
        BuildTargetPortEbpf(opts) => build_ebpf::build_ebpf("target-port-ebpf", opts),
        BuildRateLimiterEbpf(opts) => build_ebpf::build_ebpf("rate-limiter-ebpf", opts),
        BuildDnsFilterEbpf(opts) => build_ebpf::build_ebpf("dns-filter-ebpf", opts),
        BuildUdpTcpClassifierEbpf(opts) => build_ebpf::build_ebpf("udp-tcp-classifier-ebpf", opts),
        BuildEtherMirrorEbpf(opts) => build_ebpf::build_ebpf("ether-mirror-ebpf", opts),
        BuildStripEtherVlanHeaderEbpf(opts) => build_ebpf::build_ebpf("strip-ether-vlan-header-ebpf", opts),
    };

    if let Err(e) = ret {
        eprintln!("{e:#}");
        exit(1);
    }
}
