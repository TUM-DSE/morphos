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

/***
* A very simple string matching filter that drops packets that contains the bytes 'leetcodew
*/
#[inline(always)]
fn try_filter(ctx: &mut BpfContext) -> Result<FilterResult, ()> {
    let ipv4hdr: *const Ipv4Hdr = unsafe { ctx.get_ptr(0)? };

    let match_word: [u8; 8] = [
        'l' as u8,
        'e' as u8,
        'e' as u8,
        't' as u8,
        'c' as u8,
        'o' as u8,
        'd' as u8,
        'e' as u8
    ];
    let b0: u8 = 0x22;
    let b1: u8 = 0x8d;
    let mut state: u16 = 0;

    for i in 0..2000 {
        // we consider packets of size 2k at max
        let inspectable: *const u8 = match unsafe { ctx.get_ptr(i) } {
            Ok(a) => a,
            Err(_) => break, // end of packet. Stop loop
        };
        let writable: &mut u8 = unsafe { &mut *ctx.get_ptr_mut(0)? };
        let inspectable: u8 = unsafe { *inspectable };
        if state >= 8 {
            return Ok(FilterResult::Drop); // we tested and found all states (match_word bytes)
        }
        if inspectable == match_word[state as usize] {
            state += 1;
        } else {
            state = 0;
        }
    }

    Ok(FilterResult::Pass)
}
