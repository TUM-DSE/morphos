#![no_std]
#![no_main]

use aya_ebpf::bpf_printk;
use core::mem;

use network_types::eth::EthHdr;

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
    let ethhdr: *mut EthHdr = unsafe { ptr_at(data, 0)? };

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
