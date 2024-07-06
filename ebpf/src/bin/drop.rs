#![no_std]
#![no_main]

use bpf_element::filter::FilterResult;

#[no_mangle]
#[link_section = "bpffilter"]
pub extern "C" fn filter() -> FilterResult {
    FilterResult::Drop
}
