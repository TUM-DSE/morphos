#![no_std]
#![no_main]

use aya_ebpf::bpf_printk;
use core::mem;

use crate::helper::bpf_packet_add_space;

mod helper;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct RewriterContext {
    data: *const u8,
    data_end: *const u8,
}

#[no_mangle]
pub extern "C" fn rewrite(ctx: *mut RewriterContext) -> i32 {
    let mut ctx = unsafe { *ctx };

    if let Err(_) = try_rewrite(&mut ctx) {
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
fn try_rewrite(ctx: &mut RewriterContext) -> Result<(), ()> {
    let ether_type_ptr: *const u16 = unsafe { ptr_at(ctx, 12)? };
    let ether_type = u16::from_be(unsafe { *ether_type_ptr });

    const ETHERTYPE_8021Q: u16 = 0x8100;
    if ether_type == ETHERTYPE_8021Q {
        unsafe { bpf_packet_add_space(ctx, -18, 0); }
    } else {
        unsafe { bpf_packet_add_space(ctx, -14, 0); }
    }

    Ok(())
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    unsafe { core::hint::unreachable_unchecked() }
}
