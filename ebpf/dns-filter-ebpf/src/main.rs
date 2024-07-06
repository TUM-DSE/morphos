#![no_std]
#![no_main]

use aya_ebpf::bpf_printk;
use core::mem;
use aya_ebpf::bindings::xdp_action::{XDP_ABORTED, XDP_DROP, XDP_PASS};
use flex_dns::name::DnsName;
use flex_dns::{dns_name, DnsMessage};

use network_types::eth::{EthHdr, EtherType};
use network_types::ip::{IpProto, Ipv4Hdr};
use network_types::udp::UdpHdr;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct BPFilterContext {
    data: *const u8,
    data_end: *const u8,
}

#[no_mangle]
#[link_section = "bpffilter"]
pub extern "C" fn filter(ctx: *mut BPFilterContext) -> u32 {
    let ctx = unsafe { *ctx };
    match try_filter(&ctx) {
        Ok(ret) => ret,
        Err(_) => XDP_ABORTED,
    }
}

#[inline(always)]
unsafe fn ptr_at<T>(ctx: &BPFilterContext, offset: usize) -> Result<*const T, ()> {
    let start = ctx.data as usize;
    let end = ctx.data_end as usize;
    let len = mem::size_of::<T>();

    if start + offset + len > end {
        return Err(());
    }

    Ok((start + offset) as *const T)
}

#[inline(always)]
unsafe fn slice_at(ctx: &BPFilterContext, len: usize, offset: usize) -> Result<&[u8], ()> {
    let start = ctx.data as usize;
    let end = ctx.data_end as usize;

    if start + offset + len > end {
        return Err(());
    }

    let slice_start = (start + offset) as *const u8;
    Ok(core::slice::from_raw_parts(slice_start, len))
}

#[inline(always)]
fn try_filter(data: &BPFilterContext) -> Result<u32, ()> {
    let ethhdr: *const EthHdr = unsafe { ptr_at(data, 0)? };
    let ether_type = unsafe { *ethhdr }.ether_type;
    if ether_type != EtherType::Ipv4 {
        unsafe { bpf_printk!(b"not ipv4\n") };
        return Ok(XDP_PASS);
    }

    let ipv4hdr: *const Ipv4Hdr = unsafe { ptr_at(data, EthHdr::LEN)? };
    let proto = unsafe { *ipv4hdr }.proto;
    let ipv4hdr_len = unsafe { *ipv4hdr }.ihl() as usize * 4;
    if ipv4hdr_len < Ipv4Hdr::LEN {
        unsafe { bpf_printk!(b"invalid ipv4 header length\n") };
        return Ok(XDP_PASS);
    }

    // check UDP
    if proto != IpProto::Udp {
        unsafe { bpf_printk!(b"not udp\n") };
        return Ok(XDP_PASS);
    }

    // check src & dst port equal to 53
    const DNS_PORT: u16 = 53;
    let udphdr: *const UdpHdr = unsafe { ptr_at(data, EthHdr::LEN + ipv4hdr_len)? };
    let dst_port = u16::from_be(unsafe { *udphdr }.dest);
    if dst_port != DNS_PORT {
        unsafe { bpf_printk!(b"ports don't match - dst: %u\n", dst_port as u64) };
        return Ok(XDP_PASS);
    }

    // parse DNS query
    let udp_data_len = u16::from_be(unsafe { *udphdr }.len);
    let udp_data_offset = EthHdr::LEN + ipv4hdr_len + UdpHdr::LEN;
    let udp_data = unsafe { slice_at(data, udp_data_len as usize, udp_data_offset)? };

    let dns_message: DnsMessage<8, 0, _> = DnsMessage::new(udp_data).unwrap();

    let mut questions = dns_message.questions();
    let Ok(questions) = questions.iter() else {
        unsafe {
            bpf_printk!(b"error parsing dns questions\n");
        }
        return Ok(XDP_DROP);
    };

    for question in questions {
        let Ok(question) = question else {
            unsafe {
                bpf_printk!(b"error parsing dns question\n");
            }
            return Ok(XDP_DROP);
        };

        const BLOCKED: DnsName = dns_name!(b"lmu.de");
        if question.name == BLOCKED {
            unsafe {
                bpf_printk!(b"matched disallowed dns name - reject\n");
            }
            return Ok(XDP_DROP);
        }
    }

    unsafe { bpf_printk!(b"dns filtering pass\n") };

    Ok(XDP_PASS)
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    unsafe { core::hint::unreachable_unchecked() }
}
