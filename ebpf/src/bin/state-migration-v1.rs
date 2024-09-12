#![no_std]
#![no_main]

use core::ffi::{c_char, c_long};
use aya_ebpf::bpf_printk;
use aya_ebpf::macros::map;
use aya_ebpf::maps::Array;
use bpf_element::filter::FilterResult;

#[map(name = "PACKET_CTR_V1")]
static PACKET_CTR: Array<u32> = Array::with_max_entries(1, 0);

#[no_mangle]
#[link_section = "bpffilter"]
pub extern "C" fn main() -> FilterResult {
    let Some(counter) = PACKET_CTR.get_ptr_mut(0) else {
        return FilterResult::Abort;
    };

    unsafe { *counter += 1 };

    if unsafe { *counter } > 10 {
        FilterResult::Drop
    } else {
        FilterResult::Pass
    }
}
