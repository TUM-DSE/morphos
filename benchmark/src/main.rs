use std::env;
use std::net::Ipv4Addr;

use anyhow::Context;

mod startup;

pub const CLICKOS_IPV4_ADDR: Ipv4Addr = Ipv4Addr::new(172, 44, 0, 2);

fn main() -> anyhow::Result<()> {
    let subprogram = env::args()
        .nth(1)
        .context("need to pass subprogram in first argument")?;

    match subprogram.as_str() {
        "startup" => startup::run(),
        _ => {
            panic!("invalid subprogram")
        }
    }

    Ok(())
}
