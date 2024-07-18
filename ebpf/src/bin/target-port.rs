#![no_std]
#![no_main]

use bpf_element::filter::FilterResult;
use bpf_element::BpfContext;
use network_types::eth::{EthHdr, EtherType};
use network_types::ip::{IpProto, Ipv4Hdr};
use network_types::tcp::TcpHdr;
use network_types::udp::UdpHdr;

#[no_mangle]
#[link_section = "bpffilter"]
pub extern "C" fn main(ctx: *mut BpfContext) -> FilterResult {
    let ctx = unsafe { *ctx };

    try_filter(&ctx).unwrap_or_else(|_| FilterResult::Abort)
}

#[inline(always)]
fn try_filter(ctx: &BpfContext) -> Result<FilterResult, ()> {
    let ethhdr: *const EthHdr = unsafe { ctx.get_ptr(0)? };
    match unsafe { *ethhdr }.ether_type {
        EtherType::Ipv4 => {}
        _ => return Ok(FilterResult::Pass),
    }

    let ipv4hdr: *const Ipv4Hdr = unsafe { ctx.get_ptr(EthHdr::LEN)? };

    let target_port = match unsafe { *ipv4hdr }.proto {
        IpProto::Tcp => {
            let tcphdr: *const TcpHdr = unsafe { ctx.get_ptr(EthHdr::LEN + Ipv4Hdr::LEN) }?;
            u16::from_be(unsafe { *tcphdr }.dest)
        }
        IpProto::Udp => {
            let udphdr: *const UdpHdr = unsafe { ctx.get_ptr(EthHdr::LEN + Ipv4Hdr::LEN) }?;
            u16::from_be(unsafe { *udphdr }.dest)
        }
        _ => return Err(()),
    };

    if target_port == 12345 {
        Ok(FilterResult::Drop)
    } else {
        Ok(FilterResult::Pass)
    }
}
