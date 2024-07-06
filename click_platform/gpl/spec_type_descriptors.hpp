#pragma once
#include <cassert>
#include <string>
#include <unordered_map>
#include <vector>

#include "ebpf_vm_isa.hpp"

// rough estimates:
constexpr ebpf_context_descriptor_t bpfilter_descr = {16, 0, 8, -1};

extern const ebpf_context_descriptor_t bpfilter_descr;
