// Module responsible for handling *live reconfiguration* benchmarks.
// Each benchmark sub-run does following:
// 1. Triggers reconfiguration
// 2. Waits until "Reconfiguring BPFilter..." is printed => starts measurement from here (this is the start of the "downtime" of the router"
// 3. Waits until "Reconfigured BPFilter" is printed = ends measurement from here (from here on, the router is available again)
//
// Running the benchmark itself is done by running `hyperfine` on this program.
// This way we can ensure we're measuring the time until the router is really "ready" (i.e., accepts & processes packets).

use std::env;
use std::io::{BufRead, BufReader, Lines, Stdout};
use std::net::UdpSocket;
use std::process::ChildStdout;
use std::time::Instant;

use anyhow::Context;

use crate::vm::{wait_until_ready, FileSystem};
use crate::{vm, CONTROL_ADDR};

pub fn run() -> anyhow::Result<()> {
    let cpio = env::args()
        .nth(2)
        .context("need to specify the cpio archive")?;
    let bpfilter_program = env::args()
        .nth(3)
        .context("need to specify the bpfilter program")?;

    // start click VM & wait until it's ready
    let mut child = vm::start_click(
        FileSystem::CpioArchive(&cpio),
        &[
            "-netdev".to_string(),
            "bridge,id=en1,br=controlnet".to_string(),
            "-device".to_string(),
            "virtio-net-pci,netdev=en1".to_string(),
        ],
    )
        .expect("couldn't start click");

    println!("click started, waiting until ready");

    let mut lines =
        BufReader::new(child.stdout.take().expect("child stdout can't be taken")).lines();
    wait_until_ready(&mut lines);

    println!("click ready");

    // ... now we're ready to start the benchmark

    // 1. Trigger reconfiguration
    trigger_reconfiguration(&bpfilter_program)?;

    // 2. Wait until "BPF Reconfiguration (WIP)" is printed
    wait_until_reconfiguration_start(&mut lines);
    let now = Instant::now();

    // 3. Wait until "BPF Reconfiguration done (WIP)" is printed
    wait_until_reconfiguration_end(&mut lines);
    println!("Took {}ms", now.elapsed().as_millis());

    child.kill()?;

    Ok(())
}

fn trigger_reconfiguration(program: &str) -> anyhow::Result<()> {
    let mut data = Vec::new();
    data.extend_from_slice(b"control");
    data.extend_from_slice(&1u64.to_le_bytes());
    data.extend_from_slice(&(program.len() as u64).to_le_bytes());
    data.extend_from_slice(program.as_bytes());

    let socket = UdpSocket::bind("0.0.0.0:0").context("couldn't bind to control addr")?;
    socket
        .send_to(&data, CONTROL_ADDR)
        .context("couldn't send packet")?;

    Ok(())
}

fn wait_until_reconfiguration_start(lines: &mut Lines<BufReader<ChildStdout>>) {
    lines
        .filter_map(Result::ok)
        .map(|line| {
            println!("{line}");
            line
        })
        .find(|line| line.contains("Reconfiguring BPFilter..."));
}

fn wait_until_reconfiguration_end(lines: &mut Lines<BufReader<ChildStdout>>) {
    lines
        .filter_map(Result::ok)
        .map(|line| {
            println!("{line}");
            line
        })
        .find(|line| line.contains("Reconfigured BPFilter"));
}
