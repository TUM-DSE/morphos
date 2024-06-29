use core::{mem, slice};

unsafe fn bpf_packet_add_space_impl(head_len: i32, tail_len: i32) -> *mut u8 {
    let fun: unsafe extern "C" fn(head: i32, tail: i32) -> *mut u8 = mem::transmute(60usize);
    unsafe { fun(head_len, tail_len) }
}

pub unsafe fn bpf_packet_add_space<'a>(packet: &[u8], head_len: i32, tail_len: i32) -> &mut [u8] {
    let new_len = packet.len() as i32 + head_len + tail_len;
    let new_ptr = unsafe { bpf_packet_add_space_impl(head_len, tail_len) };

    unsafe { slice::from_raw_parts_mut(new_ptr, new_len as usize) }
}