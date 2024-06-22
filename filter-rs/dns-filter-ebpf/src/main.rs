#![no_std]
#![no_main]

mod helpers;

use core::mem;
use network_types::eth::{EtherType, EthHdr};
use network_types::ip::{IpProto, Ipv4Hdr};
use network_types::tcp::TcpHdr;
use network_types::udp::UdpHdr;
use crate::helpers::trace;

const DROP: u32 = 1;
const PASS: u32 = 0;

#[no_mangle]
pub extern "C" fn filter(data: *const u8, data_len: usize) -> u32 {
    let data = unsafe { core::slice::from_raw_parts(data, data_len) };

    trace(1);

    match try_filter(data) {
        Ok(ret) => ret,
        Err(_) => DROP,
    }
}

#[inline(always)]
unsafe fn ptr_at<T>(data: &[u8], offset: usize) -> Result<*const T, ()> {
    let start = data.as_ptr();
    let len = mem::size_of::<T>();

    if offset + len > data.len() {
        return Err(());
    }

    Ok(start.add(offset) as *const T)
}

fn try_filter(data: &[u8]) -> Result<u32, ()> {
    let ethhdr: *const EthHdr = unsafe { ptr_at(data, 0)? };
    match unsafe { *ethhdr }.ether_type {
        EtherType::Ipv4 => {}
        _ => return Ok(PASS),
    }

    let ipv4hdr: *const Ipv4Hdr = unsafe { ptr_at(data, EthHdr::LEN)? };

    let target_port = match unsafe { *ipv4hdr }.proto {
        IpProto::Tcp => {
            let tcphdr: *const TcpHdr =
                unsafe { ptr_at(data, EthHdr::LEN + Ipv4Hdr::LEN) }?;
            u16::from_be(unsafe { *tcphdr }.dest)
        }
        IpProto::Udp => {
            let udphdr: *const UdpHdr =
                unsafe { ptr_at(data, EthHdr::LEN + Ipv4Hdr::LEN) }?;
            u16::from_be(unsafe { *udphdr }.dest)
        }
        _ => return Err(()),
    };

    if target_port == 12345 {
        Ok(DROP)
    } else {
        Ok(PASS)
    }
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    unsafe { core::hint::unreachable_unchecked() }
}
