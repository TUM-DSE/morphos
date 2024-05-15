use std::io::{BufRead, BufReader};
use std::net::{Ipv4Addr, SocketAddrV4, UdpSocket};
use std::process::{Child, Command, exit, Stdio};
use std::{env, thread};
use std::thread::sleep;
use std::time::Duration;

fn main() {
    let mut args = env::args();
    let click_configuration = args.nth(1).expect("need to pass click configuration as first argument");
    let extra_args = args.collect::<Vec<String>>();

    thread::scope(|s| {
        s.spawn(|| {
            let mut child = start_clickos(&click_configuration, &extra_args).expect("failed to start clickos");

            // wait until the child receives one packet -> wait until the child prints "Received packet"
            let stdout = child.stdout.take().expect("failed to take stdout");
            let reader = BufReader::new(stdout);
            for line in reader.lines() {
                if let Ok(line) = line {
                    println!("{}", line);
                    if line.contains("Received packet") && !line.contains("->") {
                        println!("[Benchmark] ClickOS received packet, exiting...");
                        child.kill().unwrap();
                        exit(0);
                    }
                }
            }
        });

        s.spawn(|| {
            send_packet_loop().expect("error in send packet loop");
        });
    });
}

const CLICKOS_IPV4_ADDR: Ipv4Addr = Ipv4Addr::new(172, 44, 0, 2);

fn start_clickos(clickos_configuration: &str, extra_args: &[String]) -> anyhow::Result<Child> {
    let mut args = [
        "qemu-system-x86_64",
        "-accel", "kvm",
        "-cpu", "max",
        "-netdev", "bridge,id=en0,br=clicknet",
        "-device", "virtio-net-pci,netdev=en0",
        "-append", &format!(r#"netdev.ip={CLICKOS_IPV4_ADDR}/24:172.44.0.1 --"#),
        "-kernel", "../.unikraft/build/click_qemu-x86_64",
        "-initrd", clickos_configuration,
        "-nographic"
    ].map(|s| s.to_string()).to_vec();

    args.extend_from_slice(extra_args);

    println!("Starting ClickOS with following arguments: {args:?}");

    let child = Command::new("sudo")
        .args(args)
        .stdout(Stdio::piped())
        .spawn()?;

    Ok(child)
}

fn send_packet_loop() -> anyhow::Result<()> {
    let socket = UdpSocket::bind("127.0.0.1:0")?;

    loop {
        socket.send_to(&[0u8; 1], SocketAddrV4::new(CLICKOS_IPV4_ADDR, 1122))?;
        sleep(Duration::from_millis(1));
    }
}