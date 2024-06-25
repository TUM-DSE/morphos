use core::ffi::c_long;

pub unsafe fn bpf_trace(
    num: c_long
) {
    let fun: unsafe extern "C" fn(
        num: c_long
    ) = core::mem::transmute(4usize);
    fun(num)
}

#[inline(always)]
pub fn trace(num: i64) {
    unsafe {
        bpf_trace(num);
    }
}
