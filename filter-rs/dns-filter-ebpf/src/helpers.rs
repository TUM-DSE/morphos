use core::ffi::{c_char, c_void, CStr};

extern "C" {
    fn bpf_trace_printk(fmt: *const c_char, fmt_size: i32, ...) -> i64;
}

pub fn trace_printk(fmt: &CStr) {
    unsafe {
        let len = fmt.to_bytes().len() as i32;
        bpf_trace_printk(fmt.as_ptr(), len);
    }
}