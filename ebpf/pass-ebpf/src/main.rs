#![no_std]
#![no_main]

#[no_mangle]
pub extern "C" fn filter() -> u32 {
    return 0;
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    unsafe { core::hint::unreachable_unchecked() }
}
