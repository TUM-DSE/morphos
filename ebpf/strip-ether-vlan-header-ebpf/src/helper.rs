use core::mem;
use crate::RewriterContext;

unsafe fn bpf_packet_add_space_impl(head_len: i32, tail_len: i32) -> *mut u8 {
    let fun: unsafe extern "C" fn(head: i32, tail: i32) -> *mut u8 = mem::transmute(60usize);
    unsafe { fun(head_len, tail_len) }
}

pub unsafe fn bpf_packet_add_space<'a>(ctx: &mut RewriterContext, head_len: i32, tail_len: i32) {
    let old_len = ctx.data_end.offset_from(ctx.data) as usize;
    let new_len = old_len as isize + head_len as isize + tail_len as isize;

    let new_ptr = unsafe { bpf_packet_add_space_impl(head_len, tail_len) };
    let new_tail = new_ptr.add(new_len as usize);

    ctx.data = new_ptr;
    ctx.data_end = new_tail;
}