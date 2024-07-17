autocxx::include_cpp! {
    #include "platform.hpp"
    generate!("ebpf_platform_t")
    safety!(unsafe_ffi)
}