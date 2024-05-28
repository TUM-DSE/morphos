use std::net::UdpSocket;
use anyhow::Context;

fn main() -> anyhow::Result<()> {
    let new_program = b"filter-rs";

    let mut data = Vec::new();
    data.extend_from_slice(b"control");
    data.extend_from_slice(&1u64.to_le_bytes());
    data.extend_from_slice(&(new_program.len() as u64).to_le_bytes());
    data.extend_from_slice(new_program);

    let socket = UdpSocket::bind("0.0.0.0:0").context("couldn't bind to socket")?;
    socket.send_to(&data, "173.44.0.2:4444").context("couldn't send packet")?;

    Ok(())
}