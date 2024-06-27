#![no_std]
#![no_main]

use aya_ebpf::bpf_printk;
use core::mem;
use flex_dns::name::DnsName;
use flex_dns::{dns_name, DnsMessage};

use network_types::eth::{EthHdr, EtherType};
use network_types::ip::{IpProto, Ipv4Hdr};
use network_types::udp::UdpHdr;

const DROP: u32 = 1;
const PASS: u32 = 0;

#[no_mangle]
pub extern "C" fn filter(data: *const u8, data_len: usize) -> u32 {
    let data = unsafe { core::slice::from_raw_parts(data, data_len) };

    match try_filter(data) {
        Ok(ret) => ret,
        Err(_) => {
            unsafe {
                bpf_printk!(b"error processing packet\n");
            }
            DROP
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

fn try_filter(data: &[u8]) -> Result<u32, ()> {
    let ethhdr: *const EthHdr = unsafe { ptr_at(data, 0)? };
    let ether_type = unsafe { *ethhdr }.ether_type;
    if ether_type != EtherType::Ipv4 {
        unsafe { bpf_printk!(b"not ipv4\n") };
        return Ok(PASS);
    }

    let ipv4hdr: *const Ipv4Hdr = unsafe { ptr_at(data, EthHdr::LEN)? };
    let proto = unsafe { *ipv4hdr }.proto;
    let ipv4hdr_len = unsafe { *ipv4hdr }.ihl() as usize * 4;
    if ipv4hdr_len < Ipv4Hdr::LEN {
        unsafe { bpf_printk!(b"invalid ipv4 header length\n") };
        return Ok(PASS);
    }

    // check UDP
    if proto != IpProto::Udp {
        unsafe { bpf_printk!(b"not udp\n") };
        return Ok(PASS);
    }

    // check src & dst port equal to 53
    const DNS_PORT: u16 = 53;
    let udphdr: *const UdpHdr = unsafe { ptr_at(data, EthHdr::LEN + ipv4hdr_len)? };
    let dst_port = u16::from_be(unsafe { *udphdr }.dest);
    if dst_port != DNS_PORT {
        unsafe { bpf_printk!(b"ports don't match - dst: %u\n", dst_port as u64) };
        return Ok(PASS);
    }

    // parse DNS query
    let Some(udp_data) = ({
        let udp_data_len = u16::from_be(unsafe { *udphdr }.len);
        let udp_data_offset = EthHdr::LEN + ipv4hdr_len + UdpHdr::LEN;
        data.get(udp_data_offset..udp_data_offset + udp_data_len as usize)
    }) else {
        unsafe { bpf_printk!(b"invalid udp data length\n") };
        return Ok(DROP);
    };

    let dns_message: DnsMessage<8, 0, _> = DnsMessage::new(udp_data).unwrap();

    let mut questions = dns_message.questions();
    let Ok(questions) = questions.iter() else {
        unsafe {
            bpf_printk!(b"error parsing dns questions\n");
        }
        return Ok(DROP);
    };

    for question in questions {
        let Ok(question) = question else {
            unsafe {
                bpf_printk!(b"error parsing dns question\n");
            }
            return Ok(DROP);
        };

        const BLOCKED: DnsName = dns_name!(b"lmu.de");
        if question.name == BLOCKED {
            unsafe {
                bpf_printk!(b"matched disallowed dns name - reject\n");
            }
            return Ok(DROP);
        }
    }

    unsafe { bpf_printk!(b"dns filtering pass\n") };

    Ok(PASS)
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    unsafe { core::hint::unreachable_unchecked() }
}
