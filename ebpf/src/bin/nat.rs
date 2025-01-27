#![no_std]
#![no_main]

use aya_ebpf::cty::{c_long, c_char};
use core::mem;
use aya_ebpf::macros::map;
use aya_ebpf::helpers::bpf_printk;
use aya_ebpf::helpers::gen::bpf_ktime_get_ns;
use aya_ebpf::maps::{Array, HashMap};
use bpf_element::BpfContext;
use network_types::eth::{EthHdr, EtherType};
use network_types::ip::{IpProto, Ipv4Hdr};
use network_types::tcp::TcpHdr;
use network_types::udp::UdpHdr;

const OUTPUTS: u32 = 2;
pub type Output = u32;
const FOUTPUT: u32 = 0; // packet towards the wild
const ROUTPUT: u32 = 1; // reply flows are rewritten to look like the original flow -> routput




const FOO: Rewrite = Rewrite {
    src_ip: 0x0a000001,
    src_mac: [0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
    src_port: 0,
    dst_ip: 0x0a000002,
    dst_mac: [0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
    dst_port: 0,
};

struct Connection {
    src_ip: u32,
    src_port: u16,
    dst_ip: u32,
    dst_port: u16,
    protocol: u8,
}

struct Rewrite {
    src_ip: u32,
    src_mac: [u8; 6],
    src_port: u16,
    dst_ip: u32,
    dst_mac: [u8; 6],
    dst_port: u16
}

#[no_mangle]
#[link_section = "bpffilter"]
pub extern "C" fn main(ctx: *mut BpfContext) -> Output {
    let mut ctx = unsafe { *ctx };
    try_classify(&mut ctx).unwrap_or_else(|_| 0)
}

#[map(name = "PKTCOUNTER")]
static PKTCOUNTER: Array<u32> = Array::with_max_entries(1, 0);

#[map(name = "NEXT_PORT")]
static NEXT_PORT: Array<u16> = Array::with_max_entries(1, 0);

const PORT_START: u16 = 50000;
const PORT_END: u16 = 65535;

#[inline(always)]
fn next_port() -> Result<u16, ()> {
    let next_port = NEXT_PORT.get_ptr_mut(0).ok_or(())?;
    let port = unsafe { *next_port };
    unsafe { *next_port = (*next_port - PORT_START + 1) % (PORT_END - PORT_START) + PORT_START };
    Ok(port)
}

#[map(name = "CONNECTIONS")]
static CONNECTIONS: HashMap<Connection, u32> = HashMap::with_max_entries(1028, 0);

#[inline(always)]
fn try_classify(ctx: &mut BpfContext) -> Result<Output, ()> {
    let ethhdr: *const EthHdr = unsafe { ctx.get_ptr(0)? };
    let ether_type  = unsafe { *ethhdr }.ether_type;
    if ether_type != EtherType::Ipv4 {
        return Err(());
    }
    let ipv4hdr: *const Ipv4Hdr = unsafe { ctx.get_ptr(14)? };

    let mut conn = match unsafe { *ipv4hdr }.proto {
        IpProto::Tcp => {
            let tcphdr: *const TcpHdr = unsafe { ctx.get_ptr(Ipv4Hdr::LEN) }?;
            Connection{
                src_ip: unsafe { *ipv4hdr }.src_addr,
                src_port: u16::from_be(unsafe { *tcphdr }.source),
                dst_ip: unsafe { *ipv4hdr }.dst_addr,
                dst_port: u16::from_be(unsafe { *tcphdr }.dest),
                protocol: IpProto::Tcp as u8,
            }
        }
        IpProto::Udp => {
            let udphdr: *const UdpHdr = unsafe { ctx.get_ptr(Ipv4Hdr::LEN) }?;
            Connection{
                src_ip: unsafe { *ipv4hdr }.src_addr,
                src_port: u16::from_be(unsafe { *udphdr }.source),
                dst_ip: unsafe { *ipv4hdr }.dst_addr,
                dst_port: u16::from_be(unsafe { *udphdr }.dest),
                protocol: IpProto::Tcp as u8,
            }
        }
        _ => return Err(()),
    };

    let output = match CONNECTIONS.get_ptr(&conn) {
        Some(output) => unsafe {*output % OUTPUTS},
        None => {
            let port = next_port()? as u32;
            CONNECTIONS.insert(&conn, &port, 0).ok().ok_or(())?;
            port
        }
    };



    // let z = unsafe { bpf_ktime_get_ns() };
    // unsafe { bpf_printk!(b"foobar") };
    // let printk: unsafe extern "C" fn(fmt: *const c_char, fmt_size: u32, ...) -> c_long =
    //     unsafe { mem::transmute(6usize) };
    // let fmt = b"hello world";
    // let fmt_ptr = fmt.as_ptr() as *const c_char;
    // let fmt_size = fmt.len() as u32;
    // unsafe { printk(fmt_ptr, fmt_size) };

    // CONNECTIONS.insert(&conn, &1, 0).ok().ok_or(())?;

    let a: *const u8 = unsafe { ctx.get_ptr_mut(0)? };
    let a: u8 = unsafe { *a };
    // *a = 0xff;

    let counter = PKTCOUNTER.get_ptr_mut(0).ok_or(())?;

    let output = unsafe { *counter } % OUTPUTS;

    unsafe {
        *counter += 1;
    }
    // Ok(output)
    if conn.protocol == (IpProto::Tcp as u8) {
        Ok(1)
    } else {
        Ok(0)
    }
}
