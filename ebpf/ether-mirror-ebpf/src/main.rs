#![no_std]
#![no_main]

use aya_ebpf::bpf_printk;
use core::mem;

use network_types::eth::EthHdr;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct RewriterContext {
    data: *mut u8,
    data_end: *mut u8,
}

#[no_mangle]
#[link_section = "bpffilter"]
pub extern "C" fn rewrite(ctx: *mut RewriterContext) -> u32 {
    let ctx = unsafe { *ctx };

    if let Err(_) = try_rewrite(&ctx) {
        unsafe {
            bpf_printk!(b"error processing packet\n");
        }
    }

    0
}

#[inline(always)]
unsafe fn ptr_at<T>(ctx: &RewriterContext, offset: usize) -> Result<*mut T, ()> {
    let start = ctx.data as usize;
    let end = ctx.data_end as usize;
    let len = mem::size_of::<T>();

    if start + offset + len > end {
        return Err(());
    }

    Ok((start + offset) as *mut T)
}

#[inline(always)]
fn try_rewrite(ctx: &RewriterContext) -> Result<(), ()> {
    let ethhdr: *mut EthHdr = unsafe { ptr_at(ctx, 0)? };

    // mirror ethernet source and destination addresses
    unsafe {
        let ethhdr = &mut *ethhdr;
        mem::swap(&mut ethhdr.src_addr, &mut ethhdr.dst_addr);
    }

    Ok(())
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    unsafe { core::hint::unreachable_unchecked() }
}
