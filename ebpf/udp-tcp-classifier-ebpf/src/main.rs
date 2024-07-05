#![no_std]
#![no_main]

use aya_ebpf::bpf_printk;
use core::mem;

use network_types::eth::{EthHdr, EtherType};
use network_types::ip::{IpProto, Ipv4Hdr};

#[derive(Copy, Clone)]
enum ClassifyResult {
    Udp,
    Tcp,
    Rest,
}

impl ClassifyResult {
    pub fn output_port(self) -> u32 {
        match self {
            ClassifyResult::Udp => 0,
            ClassifyResult::Tcp => 1,
            ClassifyResult::Rest => 2,
        }
    }
}

#[no_mangle]
pub extern "C" fn classify(data: *const u8, data_len: usize) -> u32 {
    let data = unsafe { core::slice::from_raw_parts(data, data_len) };

    match try_classify(data) {
        Ok(ret) => ret.output_port(),
        Err(_) => {
            unsafe {
                bpf_printk!(b"error processing packet\n");
            }

            ClassifyResult::Rest.output_port()
        }
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

fn try_classify(data: &[u8]) -> Result<ClassifyResult, ()> {
    let ethhdr: *const EthHdr = unsafe { ptr_at(data, 0)? };
    let ether_type = unsafe { *ethhdr }.ether_type;
    if ether_type != EtherType::Ipv4 {
        unsafe { bpf_printk!(b"not ipv4\n") };
        return Ok(ClassifyResult::Rest);
    }

    let ipv4hdr: *const Ipv4Hdr = unsafe { ptr_at(data, EthHdr::LEN)? };
    let proto = unsafe { *ipv4hdr }.proto;
    let ipv4hdr_len = unsafe { *ipv4hdr }.ihl() as usize * 4;
    if ipv4hdr_len < Ipv4Hdr::LEN {
        unsafe { bpf_printk!(b"invalid ipv4 header length\n") };
        return Ok(ClassifyResult::Rest);
    }

    match proto {
        IpProto::Udp => Ok(ClassifyResult::Udp),
        IpProto::Tcp => Ok(ClassifyResult::Tcp),
        _ => Ok(ClassifyResult::Rest),
    }
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    unsafe { core::hint::unreachable_unchecked() }
}
