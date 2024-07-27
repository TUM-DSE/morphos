#![no_std]
#![no_main]

use core::net::Ipv4Addr;

use aya_ebpf::helpers::bpf_ktime_get_ns;
use aya_ebpf::macros::map;
use aya_ebpf::maps::HashMap;
use bpf_element::filter::FilterResult;
use bpf_element::BpfContext;
use network_types::ip::Ipv4Hdr;

#[no_mangle]
#[link_section = "bpffilter"]
pub extern "C" fn main(ctx: *mut BpfContext) -> FilterResult {
    let ctx = unsafe { *ctx };

   try_filter(&ctx).unwrap_or_else(|_| FilterResult::Abort)
}

struct RateLimit {
    tokens: u32,
    last_token_grant: u32,
}

impl Default for RateLimit {
    #[inline(always)]
    fn default() -> Self {
        Self {
            tokens: 1,
            last_token_grant: (unsafe { bpf_ktime_get_ns() } / 1_000_000_000) as u32,
        }
    }
}

impl RateLimit {
    #[inline(always)]
    pub fn grant_tokens_if_needed(&mut self) {
        let now = (unsafe { bpf_ktime_get_ns() } / 1_000_000_000) as u32;
        let elapsed = now - self.last_token_grant;

        // grant 1 token per second
        let new_tokens = elapsed;

        if new_tokens > 0 {
            self.tokens += new_tokens;
            self.tokens = self.tokens.min(3);
            self.last_token_grant = now;
        }
    }

    #[inline(always)]
    pub fn spend_token(&mut self) -> bool {
        if self.tokens >= 1 {
            self.tokens -= 1;
            true
        } else {
            false
        }
    }
}

#[map(name = "PKTCOUNTHASHMAP")]
static PKTCOUNTHASHMAP: HashMap<Ipv4Addr, RateLimit> = HashMap::with_max_entries(1024, 0);

#[inline(always)]
fn try_filter(ctx: &BpfContext) -> Result<FilterResult, ()> {
    let ip_hdr: *const Ipv4Hdr = unsafe { ctx.get_ptr(0)? };
    let src_addr = unsafe { (*ip_hdr).src_addr() };

    let rate_limit = match PKTCOUNTHASHMAP.get_ptr_mut(&src_addr) {
        None => {
            let rate_limit = RateLimit::default();
            PKTCOUNTHASHMAP
                .insert(&src_addr, &rate_limit, 0)
                .map_err(|_| ())?;
            unsafe { &mut *PKTCOUNTHASHMAP.get_ptr_mut(&src_addr).ok_or(())? }
        }
        Some(rate_limit) => {
            let rate_limit = unsafe { &mut *rate_limit };
            rate_limit.grant_tokens_if_needed();
            rate_limit
        }
    };

    if rate_limit.spend_token() {
        Ok(FilterResult::Pass)
    } else {
        Ok(FilterResult::Drop)
    }
}
