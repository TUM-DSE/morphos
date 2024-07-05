#![no_std]
#![no_main]

#[no_mangle]
pub extern "C" fn filter(_: *const u8, _: usize) -> u32 {
    return 1;
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    unsafe { core::hint::unreachable_unchecked() }
}
