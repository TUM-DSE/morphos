#![no_std]
#![allow(dead_code)]

mod programs;

use core::mem;

pub use programs::*;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct BpfContext {
    pub data: *mut u8,
    pub data_end: *mut u8,
    pub port: u32,
}

impl BpfContext {
    #[inline(always)]
    pub unsafe fn get_ptr<T>(&self, offset: usize) -> Result<*const T, ()> {
        let start = self.data as usize;
        let end = self.data_end as usize;
        let len = mem::size_of::<T>();

        if start + offset + len > end {
            return Err(());
        }

        Ok((start + offset) as *const T)
    }

    #[inline(always)]
    pub unsafe fn get_ptr_mut<T>(&mut self, offset: usize) -> Result<*mut T, ()> {
        let start = self.data as usize;
        let end = self.data_end as usize;
        let len = mem::size_of::<T>();

        if start + offset + len > end {
            return Err(());
        }

        Ok((start + offset) as *mut T)
    }

    #[inline(always)]
    pub unsafe fn get_slice(&self, len: usize, offset: usize) -> Result<&[u8], ()> {
        let start = self.data as usize;
        let end = self.data_end as usize;

        // Limit the size of the slice to 50KB so the verifier doesn't complain
        if len > 50000 {
            return Err(());
        }

        if start + offset + len > end {
            return Err(());
        }

        let slice_start = (start + offset) as *const u8;
        Ok(core::slice::from_raw_parts(slice_start, len))
    }
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    unsafe { core::hint::unreachable_unchecked() }
}


#[inline(always)]
pub fn update_checksum(checksum: &mut u16, old: u16, new: u16) {
    let mut sum: u32 = (!*checksum as u32).wrapping_add(!old as u32).wrapping_add(new as u32);
    sum = (sum & 0xffff).wrapping_add(sum >> 16);
    *checksum = !(sum.wrapping_add(sum >> 16)) as u16;
}

#[inline(always)]
pub fn update_checksum_ip(checksum: &mut u16, old: u32, new: u32) {
    let old1 = (old >> 16) as u16;
    let old2 = old as u16;
    let new1 = (new >> 16) as u16;
    let new2 = new as u16;
    update_checksum(checksum, old1, new1);
    update_checksum(checksum, old2, new2);
}
