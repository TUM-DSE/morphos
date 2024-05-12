use std::io::{BufRead, BufReader};
use std::net::{Ipv4Addr, SocketAddrV4, UdpSocket};
use std::process::{Child, Command, exit, Stdio};
use std::thread;
use std::thread::sleep;
use std::time::Duration;

fn main() {
    thread::scope(|s| {
        s.spawn(|| {
            let mut child = start_clickos().expect("failed to start clickos");

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
const BIND_IPV4_ADDR: SocketAddrV4 = SocketAddrV4::new(Ipv4Addr::new(0, 0, 0, 0), 1122);

fn start_clickos() -> anyhow::Result<Child> {
    let child = Command::new("sudo")
        .args([
            "qemu-system-x86_64",
            "-accel", "kvm",
            "-cpu", "max",
            "-netdev", "bridge,id=en0,br=clicknet",
            "-device", "virtio-net-pci,netdev=en0",
            "-append", &format!(r#"netdev.ip={CLICKOS_IPV4_ADDR}/24:172.44.0.1 --"#),
            "-kernel", "../.unikraft/build/click_qemu-x86_64",
            "-initrd", "minimal.click",
            "-nographic"
        ])
        .stdout(Stdio::piped())
        .spawn()?;

    Ok(child)
}

fn send_packet_loop() -> anyhow::Result<()> {
    let socket = UdpSocket::bind(BIND_IPV4_ADDR)?;

    loop {
        socket.send_to(&[0u8; 1], SocketAddrV4::new(CLICKOS_IPV4_ADDR, 1122))?;
        sleep(Duration::from_millis(1));
    }
}