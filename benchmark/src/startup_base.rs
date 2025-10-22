
use std::net::{SocketAddrV4, UdpSocket};
use crate::vm::{self, wait_until_ready, FileSystem, DATA_ADDR};
use std::time::{Duration, Instant};
use std::thread::sleep;
use std::io::BufRead;

#[derive(Clone)]
pub enum System {
    Unikraft,
    UnikraftNoPaging,
    Linux,
}

#[derive(Clone)]
pub struct Configuration<'a> {
    pub name: &'a str,
    pub click_configuration: &'a str,
    pub vm_extra_args: &'a [&'a str],
    pub system: System,
}

pub fn run_benchmark(config: &Configuration) -> Duration {
    let extra_args: Vec<_> = config.vm_extra_args.iter().map(|s| s.to_string()).collect();

    let start = Instant::now();
    let mut click_vm = match config.system {
        System::Unikraft => vm::start_click2(FileSystem::Raw(config.click_configuration), &extra_args, "../VMs/unikraft"),
        System::UnikraftNoPaging => vm::start_click2(FileSystem::Raw(config.click_configuration), &extra_args, "../VMs/unikraft_nopaging"),
        System::Linux => vm::start_linux_click(config.click_configuration, &extra_args),
    }.expect("failed to start click");

    // wait until the child receives one packet -> wait until the child prints "Received packet"
    wait_until_ready(&mut click_vm.stdout.take().unwrap().lines());

    start.elapsed()
}

pub fn send_packet_loop() -> anyhow::Result<()> {

    loop {
        let socket = UdpSocket::bind("0.0.0.0:0")?;
        socket.send_to(&[0u8; 1], SocketAddrV4::new(DATA_ADDR, 1122))?;
        sleep(Duration::from_millis(1));
    }
}
