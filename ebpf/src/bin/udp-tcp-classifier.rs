#![no_std]
#![no_main]

use bpf_element::BpfContext;
use network_types::eth::{EthHdr, EtherType};
use network_types::ip::{IpProto, Ipv4Hdr};

#[derive(Copy, Clone)]
#[repr(u32)]
pub enum ClassifyResult {
    Udp = 0,
    Tcp = 1,
    Rest = 2,
}

#[no_mangle]
#[link_section = "bpffilter"]
pub extern "C" fn main(ctx: *mut BpfContext) -> ClassifyResult {
    let ctx = unsafe { *ctx };
    try_classify(&ctx).unwrap_or_else(|_| {
        ClassifyResult::Rest
    })
}

#[inline(always)]
fn try_classify(ctx: &BpfContext) -> Result<ClassifyResult, ()> {
    let ethhdr: *const EthHdr = unsafe { ctx.get_ptr(0)? };
    let ether_type = unsafe { *ethhdr }.ether_type;
    if ether_type != EtherType::Ipv4 {
        return Ok(ClassifyResult::Rest);
    }

    let ipv4hdr: *const Ipv4Hdr = unsafe { ctx.get_ptr(EthHdr::LEN)? };
    let proto = unsafe { *ipv4hdr }.proto;
    let ipv4hdr_len = unsafe { *ipv4hdr }.ihl() as usize * 4;
    if ipv4hdr_len < Ipv4Hdr::LEN {
        return Ok(ClassifyResult::Rest);
    }

    match proto {
        IpProto::Udp => Ok(ClassifyResult::Udp),
        IpProto::Tcp => Ok(ClassifyResult::Tcp),
        _ => Ok(ClassifyResult::Rest),
    }
}
