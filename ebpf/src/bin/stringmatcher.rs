#![no_std]
#![no_main]

use bpf_element::filter::FilterResult;
use bpf_element::BpfContext;
use network_types::ip::{IpProto, Ipv4Hdr};
use network_types::tcp::TcpHdr;
use network_types::udp::UdpHdr;

#[no_mangle]
#[link_section = "bpffilter"]
pub extern "C" fn main(ctx: *mut BpfContext) -> FilterResult {
    let mut ctx = unsafe { *ctx };

    try_filter(&mut ctx).unwrap_or_else(|_| FilterResult::Abort)
}

#[inline(always)]
fn try_filter(ctx: &mut BpfContext) -> Result<FilterResult, ()> {
    let ipv4hdr: *const Ipv4Hdr = unsafe { ctx.get_ptr(0)? };

    let b0: u8 = 0x22;
    let b1: u8 = 0x8d;
    let mut state: u16 = 0;

    for i in 0..2000 { // we consider packets of size 2k at max
        let inspectable: *const u8 = match unsafe { ctx.get_ptr(i) } {
            Ok(a) => a,
            Err(_) => break, // end of packet. Stop loop
        };
        let writable: &mut u8 = unsafe { &mut *ctx.get_ptr_mut(0)? };
        let inspectable: u8 = unsafe { *inspectable };
        match state {
            0 if inspectable == b0 => { state += 1; },
            0 if inspectable != b0 => { state = 0;  },
            1 if inspectable == b1 => { state += 1; },
            1 if inspectable != b1 => { state = 0;  },
            2 => {
                return Ok(FilterResult::Drop);
                // *writable = i as u8;
                // return Ok(FilterResult::Pass);
            }
            _ => { return Err(()) }, // programmer forgot to handle a state
            // _ => { *writable = i as u8; break },
        }
    }

    Ok(FilterResult::Pass)
}
