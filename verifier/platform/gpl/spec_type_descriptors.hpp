#pragma once
#include <cassert>
#include <string>
#include <unordered_map>
#include <vector>

#include "ebpf_vm_isa.hpp"

// rough estimates:
constexpr ebpf_context_descriptor_t bpfilter_descr = {
    .size = 8 + 8 + 4, // data start and end pointer + 4 bytes for our input port number
    .data = 0,
    .end = 8,
    .meta = -1
};

extern const ebpf_context_descriptor_t bpfilter_descr;
