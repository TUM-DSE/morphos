use std::env::args;
use std::net::UdpSocket;
use anyhow::{bail, Context};

fn main() -> anyhow::Result<()> {
    let arg = args().nth(1);
    match arg.as_deref() {
        Some("reconfigure") => {
            reconfigure()?;
        }
        Some("send-packet") => {
            send_packet()?;
        }
        _ => bail!("Invalid argument")
    }

    Ok(())
}

const CONTROL_ADDR: &str = "173.44.0.2:4444";
const DATA_ADDR: &str = "172.44.0.2:4444";

fn reconfigure() -> anyhow::Result<()> {
    let new_program = args().nth(2).context("new program needs to be passed")?;

    let mut data = Vec::new();
    data.extend_from_slice(b"control");
    data.extend_from_slice(&1u64.to_le_bytes());
    data.extend_from_slice(&(new_program.len() as u64).to_le_bytes());
    data.extend_from_slice(new_program.as_bytes());

    socket()?.send_to(&data, CONTROL_ADDR).context("couldn't send packet")?;

    Ok(())
}

fn send_packet() -> anyhow::Result<()> {
    socket()?.send_to(b"data", DATA_ADDR).context("couldn't send packet")?;

    Ok(())
}

fn socket() -> anyhow::Result<UdpSocket> {
    UdpSocket::bind("0.0.0.0:0").context("couldn't bind to socket")
}