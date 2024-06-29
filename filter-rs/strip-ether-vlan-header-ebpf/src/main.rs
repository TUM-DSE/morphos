#![no_std]
#![no_main]

use aya_ebpf::bpf_printk;
use core::mem;
use network_types::eth::{EtherType, EthHdr};

use crate::helper::bpf_packet_add_space;

mod helper;

#[no_mangle]
pub extern "C" fn rewrite(data: *mut u8, data_len: usize) -> i32 {
    let data = unsafe { core::slice::from_raw_parts_mut(data, data_len) };

    if let Err(_) = try_rewrite(data) {
        unsafe {
            bpf_printk!(b"error processing packet\n");
        }
    }

    0
}

#[inline(always)]
unsafe fn ptr_at<T>(data: &mut [u8], offset: usize) -> Result<*mut T, ()> {
    let start = data.as_ptr();
    let len = mem::size_of::<T>();

    if offset + len > data.len() {
        return Err(());
    }

    Ok(start.add(offset) as *mut T)
}

fn try_rewrite(data: &mut [u8]) -> Result<(), ()> {
    let ether_type_ptr: *const u16 = unsafe { ptr_at(data, 12)? };
    let ether_type = u16::from_be(unsafe { *ether_type_ptr });

    const ETHERTYPE_8021Q: u16 = 0x8100;
    if ether_type == ETHERTYPE_8021Q {
        if data.len() > 18 {
            unsafe { bpf_packet_add_space(data, -18, 0); }
        }
    } else {
        if data.len() > 14 {
            unsafe { bpf_packet_add_space(data, -14, 0); }
        }
    }

    Ok(())
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    unsafe { core::hint::unreachable_unchecked() }
}
