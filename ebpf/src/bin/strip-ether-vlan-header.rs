#![no_std]
#![no_main]

use aya_ebpf::bpf_printk;
use bpf_element::rewriter::bpf_packet_add_space;
use bpf_element::BpfContext;

#[no_mangle]
pub extern "C" fn rewrite(ctx: *mut BpfContext) -> i32 {
    let mut ctx = unsafe { *ctx };

    if let Err(_) = try_rewrite(&mut ctx) {
        unsafe {
            bpf_printk!(b"error processing packet\n");
        }
    }

    0
}

#[inline(always)]
fn try_rewrite(ctx: &mut BpfContext) -> Result<(), ()> {
    let ether_type_ptr: *const u16 = unsafe { ctx.get_ptr(12)? };
    let ether_type = u16::from_be(unsafe { *ether_type_ptr });

    const ETHERTYPE_8021Q: u16 = 0x8100;
    if ether_type == ETHERTYPE_8021Q {
        unsafe {
            bpf_packet_add_space(ctx, -18, 0);
        }
    } else {
        unsafe {
            bpf_packet_add_space(ctx, -14, 0);
        }
    }

    Ok(())
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    unsafe { core::hint::unreachable_unchecked() }
}
