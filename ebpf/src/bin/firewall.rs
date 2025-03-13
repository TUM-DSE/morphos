#![no_std]
#![no_main]

use aya_ebpf::cty::{c_long, c_char};
use core::mem;
use aya_ebpf::macros::map;
use aya_ebpf::helpers::bpf_printk;
use aya_ebpf::helpers::gen::bpf_ktime_get_ns;
use aya_ebpf::maps::{Array, HashMap};
use bpf_element::{BpfContext, update_checksum_ip};
use network_types::eth::{EthHdr, EtherType};
use network_types::ip::{IpProto, Ipv4Hdr};
use network_types::tcp::TcpHdr;
use network_types::udp::UdpHdr;
use bpf_element::filter::FilterResult;

const PACKET_START: usize = 0; // 14 if ethernet has not been stripped
//
#[no_mangle]
#[link_section = "bpffilter"]
pub extern "C" fn main(ctx: *mut BpfContext) -> FilterResult {
    let mut ctx = unsafe { *ctx };
    // unsafe { bpf_printk!(b"port %d\n", ctx.port) };
    try_classify(&mut ctx).unwrap_or_else(|_| FilterResult::Drop)
}

#[inline(always)]
fn try_classify(ctx: &mut BpfContext) -> Result<FilterResult, ()> {
    let ipv4hdr: *mut Ipv4Hdr = unsafe { ctx.get_ptr_mut(PACKET_START)? };
    let proto = unsafe { *ipv4hdr }.proto;
    let (src_port, dst_port) = match proto {
        IpProto::Tcp => {
            // unsafe { bpf_printk!(b"foo #2.1\n") };
            let tcphdr: *const TcpHdr = unsafe { ctx.get_ptr(PACKET_START + Ipv4Hdr::LEN) }?;
            (u16::from_be(unsafe { *tcphdr }.source), u16::from_be(unsafe { *tcphdr }.dest))
        }
        IpProto::Udp => {
            let udphdr: *const UdpHdr = unsafe { ctx.get_ptr(PACKET_START + Ipv4Hdr::LEN) }?;
            (u16::from_be(unsafe { *udphdr}.source), u16::from_be(unsafe { *udphdr }.dest))
        }
        _ => {
            // unsafe { bpf_printk!(b"err! #1\n") };
            return Err(())
        },
    };

    // in vim, mark a block of numbers and increment them sequentially with g<C-a> (vim may hang
    // for some time)
    Ok(match dst_port {
        // start
        // end
        _ => FilterResult::Drop,
    })
}
