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

// NAT ports
const PORT_START: u16 = 50000;
const PORT_END: u16 = 65535;

const FOUTPUT: u32 = 0; // packet towards the wild. Will have src_ip == DEV_EX.ip and dst_mac == GW_ADDR.mac.
const ROUTPUT: u32 = 1; // reply flows are rewritten to look like the original flow -> routput (to the internal network)

const PACKET_START: usize = 0; // 14 if ethernet has not been stripped

const DEV_IN: InterfaceInfo = InterfaceInfo {
    // mac: [0x00, 0x0d, 0x87, 0x9d, 0x1c, 0xe9],
    ip: 0xac2c0002_u32.to_be(), // 172.44.0.2
    subnet: 24,
};
const DEV_EX: InterfaceInfo = InterfaceInfo {
    // mac: [0x00, 0x0d, 0x87, 0x9d, 0x1c, 0xe9],
    ip: 0xac2c0003_u32.to_be(), // 172.44.0.3
    subnet: 0,
};
const GW_ADDR: InterfaceInfo = InterfaceInfo {
    // mac: [0x00, 0x20, 0x6f, 0x9d, 0x1c, 0xc2],
    ip: 0xac2c0001_u32.to_be(), // 172.44.0.2
    subnet: 0,
};

struct Connection {
    src_ip: u32,
    src_port: u16,
    dst_ip: u32,
    dst_port: u16,
    protocol: IpProto,
}

struct Rewrite {
    src_ip: u32,
    // src_mac: [u8; 6],
    src_port: u16,
    dst_ip: u32,
    // dst_mac: [u8; 6],
    dst_port: u16,
    output: Output
}

struct InterfaceInfo {
    // mac: [u8; 6],
    ip: u32,
    subnet: u8,
}

#[no_mangle]
#[link_section = "bpffilter"]
pub extern "C" fn main(ctx: *mut BpfContext) -> Output {
    let mut ctx = unsafe { *ctx };
    // unsafe { bpf_printk!(b"port %d\n", ctx.port) };
    try_classify(&mut ctx).unwrap_or_else(|_| 0)
}

#[map(name = "PKTCOUNTER")]
static PKTCOUNTER: Array<u32> = Array::with_max_entries(1, 0);

#[map(name = "NEXT_PORT")]
static NEXT_PORT: Array<u32> = Array::with_max_entries(1, 0);

#[inline(always)]
fn next_port() -> Result<u16, ()> {
    let next_port = NEXT_PORT.get_ptr_mut(0).ok_or(())?;
    let port = unsafe { *next_port };
    // unsafe { bpf_printk!(b"next_port %d\n", port) };
    if port == 0 {
        // unsafe { bpf_printk!(b"set 0\n") };
        unsafe { *next_port = PORT_START as u32 };
    }
    let port = unsafe { *next_port };
    // unsafe { bpf_printk!(b"next_port %d\n", port) };
    // unsafe { bpf_printk!(b"next_port %d\n", *next_port) };
    unsafe { *next_port = (*next_port - PORT_START as u32 + 1) % (PORT_END - PORT_START) as u32 + PORT_START as u32 };
    Ok(port as u16)
}

#[inline(always)]
fn apply_rewrite(ctx: &mut BpfContext, conn: &Connection, rewrite: *const Rewrite) -> Result<(), ()> {
    let ipv4hdr: *mut Ipv4Hdr = unsafe { ctx.get_ptr_mut(PACKET_START)? };
    unsafe { (*ipv4hdr).src_addr = (*rewrite).src_ip };
    unsafe { (*ipv4hdr).dst_addr = (*rewrite).dst_ip };
    match conn.protocol {
        IpProto::Tcp => {
            let tcphdr: *mut TcpHdr = unsafe { ctx.get_ptr_mut(PACKET_START + Ipv4Hdr::LEN) }?;
            unsafe { (*tcphdr).source = (*rewrite).src_port.to_be() };
            unsafe { (*tcphdr).dest = (*rewrite).dst_port.to_be() };
        }
        IpProto::Udp => {
            let udphdr: *mut UdpHdr = unsafe { ctx.get_ptr_mut(PACKET_START + Ipv4Hdr::LEN) }?;
            unsafe { (*udphdr).source = (*rewrite).src_port.to_be() };
            unsafe { (*udphdr).dest = (*rewrite).dst_port.to_be() };
        }
        _ => {
            unsafe { bpf_printk!(b"err! #4\n") };
            return Err(())
        }
    }
    Ok(())
}

#[map(name = "CONNECTIONS")]
static CONNECTIONS: HashMap<Connection, Rewrite> = HashMap::with_max_entries(1028, 0);

#[inline(always)]
fn try_classify(ctx: &mut BpfContext) -> Result<Output, ()> {
    let port = ctx.port as u32;
    // let ethhdr: *const EthHdr = unsafe { ctx.get_ptr(0)? };
    // let ether_type  = unsafe { *ethhdr }.ether_type;
    // if ether_type != EtherType::Ipv4 {
    //     unsafe { bpf_printk!(b"err! #2\n") };
    //     return Err(());
    // }
    let ipv4hdr: *mut Ipv4Hdr = unsafe { ctx.get_ptr_mut(PACKET_START)? };
    let a = 1337;
    let b: *const u32 = &a;

    let proto = unsafe { *ipv4hdr }.proto;
    // unsafe { bpf_printk!(b"proto %d\n", proto as u8) };

    let mut conn = match proto {
        IpProto::Tcp => {
            unsafe { bpf_printk!(b"foo #2.1\n") };
            let tcphdr: *const TcpHdr = unsafe { ctx.get_ptr(PACKET_START + Ipv4Hdr::LEN) }?;
            Connection{
                src_ip: unsafe { *ipv4hdr }.src_addr,
                src_port: u16::from_be(unsafe { *tcphdr }.source),
                dst_ip: unsafe { *ipv4hdr }.dst_addr,
                dst_port: u16::from_be(unsafe { *tcphdr }.dest),
                protocol: IpProto::Tcp,
            }
        }
        IpProto::Udp => {
            let udphdr: *const UdpHdr = unsafe { ctx.get_ptr(PACKET_START + Ipv4Hdr::LEN) }?;
            // unsafe { bpf_printk!(b"foo #3\n") };
            Connection{
                src_ip: unsafe { *ipv4hdr }.src_addr,
                src_port: u16::from_be(unsafe { *udphdr }.source),
                dst_ip: unsafe { *ipv4hdr }.dst_addr,
                dst_port: u16::from_be(unsafe { *udphdr }.dest),
                protocol: IpProto::Udp,
            }
        }
        _ => {
            unsafe { bpf_printk!(b"err! #1\n") };
            return Err(())
        },
    };

    // handles packets from internal network for external network
    let output = match CONNECTIONS.get_ptr(&conn) {
        Some(rewrite) => {
            // unsafe { bpf_printk!(b"rewrite port %d\n", (*rewrite).src_port) };
            apply_rewrite(ctx, &conn, rewrite)?;

            unsafe {(*rewrite).output % OUTPUTS}
        },

        None if port == 1 => { FOUTPUT },

        None if port == 0 => {
            let local_nat_port = next_port()? as u32;

            // install outgoing rewrite rule (into the wild)
            let key_to = &conn;
            let value_to = Rewrite {
                src_ip: DEV_EX.ip,
                src_port: local_nat_port as u16,
                dst_ip: conn.dst_ip,
                dst_port: conn.dst_port,
                output: FOUTPUT,
            };
            // unsafe { bpf_printk!(b"local_nat_port %d\n", local_nat_port) };
            CONNECTIONS.insert(key_to, &value_to, 0).ok().ok_or(())?;

            // install incoming rewrite rule (replies from the wild)
            let key_from = Connection {
                src_ip: conn.dst_ip,
                src_port: conn.dst_port,
                dst_ip: DEV_EX.ip,
                dst_port: local_nat_port as u16,
                protocol: conn.protocol,
            };
            let value_from = Rewrite {
                src_ip: conn.dst_ip,
                src_port: conn.dst_port,
                dst_ip: conn.src_ip,
                dst_port: conn.dst_port,
                output: FOUTPUT,
            };
            CONNECTIONS.insert(&key_from, &value_from, 0).ok().ok_or(())?;
            apply_rewrite(ctx, &conn, &value_to)?;

            port
        },
        None => { // catch remaining None cases
            unsafe { bpf_printk!(b"err! #3\n") };
            return Err(())
        }
    };
    // unsafe { bpf_printk!(b"port %d #2\n", output) };

    Ok(output)


    // let z = unsafe { bpf_ktime_get_ns() };
    // unsafe { bpf_printk!(b"foobar") };
    // let printk: unsafe extern "C" fn(fmt: *const c_char, fmt_size: u32, ...) -> c_long =
    //     unsafe { mem::transmute(6usize) };
    // let fmt = b"hello world";
    // let fmt_ptr = fmt.as_ptr() as *const c_char;
    // let fmt_size = fmt.len() as u32;
    // unsafe { printk(fmt_ptr, fmt_size) };

    // CONNECTIONS.insert(&conn, &1, 0).ok().ok_or(())?;

    // let a: *const u8 = unsafe { ctx.get_ptr_mut(0)? };
    // let a: u8 = unsafe { *a };
    // // *a = 0xff;

    // let counter = PKTCOUNTER.get_ptr_mut(0).ok_or(())?;

    // let output = unsafe { *counter } % OUTPUTS;

    // unsafe {
    //     *counter += 1;
    // }
    // // Ok(output)
    // if conn.protocol == (IpProto::Tcp as u8) {
    //     Ok(1)
    // } else {
    //     Ok(0)
    // }
}
