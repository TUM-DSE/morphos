#![no_std]
#![no_main]

use core::mem;
use aya_ebpf::bindings::xdp_action::{XDP_ABORTED, XDP_DROP, XDP_PASS};
use network_types::eth::{EtherType, EthHdr};
use network_types::ip::{IpProto, Ipv4Hdr};
use network_types::tcp::TcpHdr;
use network_types::udp::UdpHdr;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct BPFilterContext {
    data: *const u8,
    data_end: *const u8,
}

#[no_mangle]
#[link_section = "bpffilter"]
pub extern "C" fn filter(ctx: *mut BPFilterContext) -> u32 {
    let ctx = unsafe { *ctx };

    match try_filter(&ctx) {
        Ok(ret) => ret,
        Err(_) => XDP_ABORTED,
    }
}

#[inline(always)]
unsafe fn ptr_at<T>(ctx: &BPFilterContext, offset: usize) -> Result<*const T, ()> {
    let start = ctx.data as usize;
    let end = ctx.data_end as usize;
    let len = mem::size_of::<T>();

    if start + offset + len > end {
        return Err(());
    }

    Ok((start + offset) as *const T)
}

#[inline(always)]
fn try_filter(ctx: &BPFilterContext) -> Result<u32, ()> {
    let ethhdr: *const EthHdr = unsafe { ptr_at(ctx, 0)? };
    match unsafe { *ethhdr }.ether_type {
        EtherType::Ipv4 => {}
        _ => return Ok(XDP_PASS),
    }

    let ipv4hdr: *const Ipv4Hdr = unsafe { ptr_at(ctx, EthHdr::LEN)? };

    let target_port = match unsafe { *ipv4hdr }.proto {
        IpProto::Tcp => {
            let tcphdr: *const TcpHdr =
                unsafe { ptr_at(ctx, EthHdr::LEN + Ipv4Hdr::LEN) }?;
            u16::from_be(unsafe { *tcphdr }.dest)
        }
        IpProto::Udp => {
            let udphdr: *const UdpHdr =
                unsafe { ptr_at(ctx, EthHdr::LEN + Ipv4Hdr::LEN) }?;
            u16::from_be(unsafe { *udphdr }.dest)
        }
        _ => return Err(()),
    };

    if target_port == 12345 {
        Ok(XDP_DROP)
    } else {
        Ok(XDP_PASS)
    }
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    unsafe { core::hint::unreachable_unchecked() }
}
