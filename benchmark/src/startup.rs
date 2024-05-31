// Module responsible for handling *startup* benchmarks.
// This module doesn't run benchmarks itself, it merely provides a CLI helper interface which does following:
// 1. Start qemu with the appropriate arguments
// 2. Exit the program once "Received packet" was printed to qemu's stdout
// 3. In a separate thread, continuously send a lot of UDP packets to qemu (in order to trigger the "Received packet" message).
//
// Running the benchmark itself is done by running `hyperfine` on this program.
// This way we can ensure we're measuring the time until the router is really "ready" (i.e., accepts & processes packets).

use crate::vm;
use crate::vm::{wait_until_ready, FileSystem};
use anyhow::Context;
use std::io::{BufRead, BufReader};
use std::net::{SocketAddrV4, UdpSocket};
use std::process::exit;
use std::thread::sleep;
use std::time::Duration;
use std::{env, thread};

pub fn run() -> anyhow::Result<()> {
    let mut args = env::args();
    let click_configuration = args
        .nth(2)
        .context("need to pass click configuration as second argument")?;
    let extra_args = args.collect::<Vec<String>>();

    thread::scope(|s| {
        s.spawn(|| {
            let mut child = vm::start_click(FileSystem::Raw(&click_configuration), &extra_args)
                .expect("failed to start clickos");

            // wait until the child receives one packet -> wait until the child prints "Received packet"
            let stdout = child.stdout.take().expect("failed to take stdout");
            let reader = BufReader::new(stdout);
            wait_until_ready(&mut reader.lines());

            child.kill().unwrap();
            exit(0);
        });

        s.spawn(|| {
            send_packet_loop().expect("error in send packet loop");
        });
    });

    Ok(())
}

fn send_packet_loop() -> anyhow::Result<()> {
    let socket = UdpSocket::bind("127.0.0.1:0")?;

    loop {
        socket.send_to(&[0u8; 1], SocketAddrV4::new(crate::DATA_ADDR, 1122))?;
        sleep(Duration::from_millis(1));
    }
}
