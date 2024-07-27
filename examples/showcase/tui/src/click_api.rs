use std::net::UdpSocket;
use eyre::Context;

const CONTROL_ADDR: &str = "173.44.0.2:4444";
const DATA_ADDR: &str = "172.44.0.2:4444";

pub struct ClickApi {
    socket: UdpSocket,
}

impl ClickApi {
    pub fn new() -> eyre::Result<Self> {
        let socket = UdpSocket::bind("0.0.0.0:0").context("couldn't bind to UDP socket")?;

        Ok(Self { socket })
    }

    pub fn reconfigure(&self, bpfilter_id: u64, program: &str, signature: &str) -> eyre::Result<()> {
        let mut data = Vec::new();
        data.extend_from_slice(b"control");
        data.extend_from_slice(&bpfilter_id.to_le_bytes());
        data.extend_from_slice(&(program.len() as u64).to_le_bytes());
        data.extend_from_slice(program.as_bytes());
        data.extend_from_slice(&(signature.len() as u64).to_le_bytes());
        data.extend_from_slice(signature.as_bytes());

        self.socket.send_to(&data, CONTROL_ADDR).context("couldn't send packet")?;

        Ok(())
    }

    pub fn send_data_packet(&self) -> eyre::Result<()> {
        self.socket.send_to(b"data", DATA_ADDR).context("couldn't send packet")?;

        Ok(())
    }
}