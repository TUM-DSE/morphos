#![no_std]
#![no_main]

use aya_ebpf::bpf_printk;
use flex_dns::name::DnsName;
use flex_dns::{dns_name, DnsMessage};

use bpf_element::filter::FilterResult;
use bpf_element::BpfContext;
use network_types::eth::{EthHdr, EtherType};
use network_types::ip::{IpProto, Ipv4Hdr};
use network_types::udp::UdpHdr;

#[no_mangle]
#[link_section = "bpffilter"]
pub extern "C" fn filter(ctx: *mut BpfContext) -> FilterResult {
    let ctx = unsafe { *ctx };
    try_filter(&ctx).unwrap_or_else(|_| FilterResult::Abort)
}

#[inline(always)]
fn try_filter(ctx: &BpfContext) -> Result<FilterResult, ()> {
    let ethhdr: *const EthHdr = unsafe { ctx.get_ptr(0)? };
    let ether_type = unsafe { *ethhdr }.ether_type;
    if ether_type != EtherType::Ipv4 {
        unsafe { bpf_printk!(b"not ipv4\n") };
        return Ok(FilterResult::Pass);
    }

    let ipv4hdr: *const Ipv4Hdr = unsafe { ctx.get_ptr(EthHdr::LEN)? };
    let proto = unsafe { *ipv4hdr }.proto;
    let ipv4hdr_len = unsafe { *ipv4hdr }.ihl() as usize * 4;
    if ipv4hdr_len < Ipv4Hdr::LEN {
        unsafe { bpf_printk!(b"invalid ipv4 header length\n") };
        return Ok(FilterResult::Pass);
    }

    // check UDP
    if proto != IpProto::Udp {
        unsafe { bpf_printk!(b"not udp\n") };
        return Ok(FilterResult::Pass);
    }

    // check src & dst port equal to 53
    const DNS_PORT: u16 = 53;
    let udphdr: *const UdpHdr = unsafe { ctx.get_ptr(EthHdr::LEN + ipv4hdr_len)? };
    let dst_port = u16::from_be(unsafe { *udphdr }.dest);
    if dst_port != DNS_PORT {
        unsafe { bpf_printk!(b"ports don't match - dst: %u\n", dst_port as u64) };
        return Ok(FilterResult::Pass);
    }

    // parse DNS query
    let udp_data_len = u16::from_be(unsafe { *udphdr }.len);
    let udp_data_offset = EthHdr::LEN + ipv4hdr_len + UdpHdr::LEN;
    let udp_data = unsafe { ctx.get_slice(udp_data_len as usize, udp_data_offset)? };

    let dns_message: DnsMessage<8, 0, _> = DnsMessage::new(udp_data).unwrap();

    let mut questions = dns_message.questions();
    let Ok(questions) = questions.iter() else {
        unsafe {
            bpf_printk!(b"error parsing dns questions\n");
        }
        return Ok(FilterResult::Drop);
    };

    for question in questions {
        let Ok(question) = question else {
            unsafe {
                bpf_printk!(b"error parsing dns question\n");
            }
            return Ok(FilterResult::Drop);
        };

        const BLOCKED: DnsName = dns_name!(b"lmu.de");
        if question.name == BLOCKED {
            unsafe {
                bpf_printk!(b"matched disallowed dns name - reject\n");
            }
            return Ok(FilterResult::Drop);
        }
    }

    unsafe { bpf_printk!(b"dns filtering pass\n") };

    Ok(FilterResult::Pass)
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    unsafe { core::hint::unreachable_unchecked() }
}
