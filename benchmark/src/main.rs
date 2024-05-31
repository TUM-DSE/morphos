use std::env;
use std::net::{Ipv4Addr, SocketAddrV4};

use anyhow::Context;

mod live_reconfigure;
mod startup;
pub mod vm;

pub const DATA_ADDR: Ipv4Addr = Ipv4Addr::new(172, 44, 0, 2);
pub const CONTROL_ADDR: SocketAddrV4 = SocketAddrV4::new(Ipv4Addr::new(173, 44, 0, 2), 4444);

fn main() -> anyhow::Result<()> {
    let subprogram = env::args()
        .nth(1)
        .context("need to pass subprogram in first argument")?;

    match subprogram.as_str() {
        "startup" => startup::run(),
        "reconfigure" => live_reconfigure::run(),
        _ => {
            panic!("invalid subprogram")
        }
    }
}
