#![no_std]
#![no_main]

use aya_ebpf::macros::map;
use aya_ebpf::maps::Array;
use bpf_element::BpfContext;

const OUTPUTS: u32 = 2;
pub type Output = u32;

#[no_mangle]
#[link_section = "bpffilter"]
pub extern "C" fn main(ctx: *mut BpfContext) -> Output {
    let ctx = unsafe { *ctx };
    try_classify(&ctx).unwrap_or_else(|_| 0)
}

#[map(name = "PKTCOUNTER")]
static PKTCOUNTER: Array<u32> = Array::with_max_entries(1, 0);

#[inline(always)]
fn try_classify(_: &BpfContext) -> Result<Output, ()> {
    let counter = PKTCOUNTER.get_ptr_mut(0).ok_or(())?;

    let output = unsafe { *counter } % OUTPUTS;

    unsafe {
        *counter += 1;
    }
    Ok(output)
}
