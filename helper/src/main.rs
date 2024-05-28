use std::net::UdpSocket;
use anyhow::Context;

fn main() -> anyhow::Result<()> {
    let socket = UdpSocket::bind("0.0.0.0:0").context("couldn't bind to socket")?;
    socket.send_to(b"control", "173.44.0.2:4444").context("couldn't send packet")?;

    Ok(())
}