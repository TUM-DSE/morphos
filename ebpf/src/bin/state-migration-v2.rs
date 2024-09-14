#![no_std]
#![no_main]

use aya_ebpf::macros::map;
use aya_ebpf::maps::Array;
use bpf_element::filter::FilterResult;

#[map(name = "PACKET_CTR_V2")]
static PACKET_CTR_V2: Array<u64> = Array::with_max_entries(1, 0);

#[no_mangle]
#[link_section = "bpffilter"]
pub extern "C" fn main() -> FilterResult {
    if let Err(_) = handle_migration() {
        return FilterResult::Abort;
    }

    let Some(counter) = PACKET_CTR_V2.get_ptr_mut(0) else {
        return FilterResult::Abort;
    };

    unsafe { *counter += 1 };

    let current_count = unsafe { *counter };
    if current_count > 10 {
        FilterResult::Drop
    } else {
        FilterResult::Pass
    }
}

#[map(name = "PACKET_CTR_V1")]
static PACKET_CTR_V1: Array<u32> = Array::with_max_entries(1, 0);

#[map(name = "VERSION")]
static VERSION: Array<u64> = Array::with_max_entries(1, 0);

fn handle_migration() -> Result<(), ()> {
    let current_version = VERSION.get_ptr_mut(0).ok_or(())?;

    if unsafe { *current_version } != 2 {
        let old_counter = PACKET_CTR_V1.get(0).ok_or(())?;
        let new_counter = PACKET_CTR_V2.get_ptr_mut(0).ok_or(())?;

        unsafe {
            *new_counter = *old_counter as u64;
            *current_version = 2;
        };
    }

    Ok(())
}
