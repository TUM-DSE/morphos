#![no_std]
#![no_main]

use bpf_element::filter::FilterResult;

#[no_mangle]
pub extern "C" fn filter() -> FilterResult {
    FilterResult::Pass
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    unsafe { core::hint::unreachable_unchecked() }
}
