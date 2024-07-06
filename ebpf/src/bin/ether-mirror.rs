#![no_std]
#![no_main]

use core::mem;

use bpf_element::BpfContext;
use network_types::eth::EthHdr;
use bpf_element::rewriter::RewriterResult;

#[no_mangle]
#[link_section = "bpffilter"]
pub extern "C" fn rewrite(ctx: *mut BpfContext) -> RewriterResult {
    let mut ctx = unsafe { *ctx };

    match try_rewrite(&mut ctx) {
        Ok(_) => RewriterResult::Success,
        Err(_) => RewriterResult::Abort,
    }
}

#[inline(always)]
fn try_rewrite(ctx: &mut BpfContext) -> Result<(), ()> {
    let ethhdr: &mut EthHdr = unsafe { &mut *ctx.get_ptr_mut(0)? };

    // mirror ethernet source and destination addresses
    mem::swap(&mut ethhdr.dst_addr, &mut ethhdr.src_addr);

    Ok(())
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    unsafe { core::hint::unreachable_unchecked() }
}
