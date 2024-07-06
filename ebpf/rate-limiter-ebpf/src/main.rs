#![no_std]
#![no_main]

use core::mem;
use core::net::Ipv4Addr;
use aya_ebpf::bindings::xdp_action::{XDP_ABORTED, XDP_DROP, XDP_PASS};
use aya_ebpf::bpf_printk;

use aya_ebpf::helpers::bpf_ktime_get_ns;
use aya_ebpf::macros::map;
use aya_ebpf::maps::HashMap;
use network_types::eth::{EtherType, EthHdr};
use network_types::ip::Ipv4Hdr;

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

struct RateLimit {
    tokens: u32,
    last_token_grant: u64,
}

impl Default for RateLimit {
    #[inline(always)]
    fn default() -> Self {
        Self {
            tokens: 3,
            last_token_grant: unsafe { bpf_ktime_get_ns() },
        }
    }
}

impl RateLimit {
    #[inline(always)]
    pub fn grant_tokens_if_needed(&mut self) {
        let now = unsafe { bpf_ktime_get_ns() };
        let elapsed = now - self.last_token_grant;

        // grant 1 token per second
        let new_tokens = elapsed / 1_000_000_000;

        if new_tokens > 0 {
            self.tokens += new_tokens as u32;
            self.tokens = self.tokens.min(3);
            self.last_token_grant = now;
        }
    }

    #[inline(always)]
    pub fn spend_token(&mut self) -> bool {
        if self.tokens >= 2 {
            self.tokens -= 2;
            true
        } else {
            false
        }
    }
}

#[map(name = "PKTCOUNTHASHMAP")]
static PKTCOUNTHASHMAP: HashMap<Ipv4Addr, RateLimit> = HashMap::with_max_entries(1024, 0);

#[inline(always)]
fn try_filter(ctx: &BPFilterContext) -> Result<u32, ()> {
    let ethhdr: *const EthHdr = unsafe { ptr_at(ctx, 0)? };
    match unsafe { *ethhdr }.ether_type {
        EtherType::Ipv4 => {}
        _ => return Ok(XDP_DROP),
    }

    let ipv4hdr: *const Ipv4Hdr = unsafe { ptr_at(ctx, EthHdr::LEN)? };
    let src_addr = unsafe { *ipv4hdr }.src_addr();

    let rate_limit = match PKTCOUNTHASHMAP.get_ptr_mut(&src_addr) {
        None => {
            let rate_limit = RateLimit::default();

            PKTCOUNTHASHMAP.insert(&src_addr, &rate_limit, 0).map_err(|_| ())?;
            unsafe { &mut *PKTCOUNTHASHMAP.get_ptr_mut(&src_addr).unwrap() }
        }
        Some(rate_limit) => {
            let rate_limit = unsafe { &mut *rate_limit };
            rate_limit.grant_tokens_if_needed();
            rate_limit
        }
    };

    unsafe { bpf_printk!(b"Currently have %d tokens\n", rate_limit.tokens); }

    if rate_limit.spend_token() {
        Ok(XDP_PASS)
    } else {
        Ok(XDP_DROP)
    }
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    unsafe { core::hint::unreachable_unchecked() }
}
