#include "bpf_helpers.hh"

#include <cstdarg>
#include <cstdio>
#include <cstdint>

long bpf_trace_printk(const char *fmt, int fmt_size, ...) {
    va_list args;
    va_start(args, fmt_size);
    std::vprintf(fmt, args);
    va_end(args);
    return 0;
}

void *bpf_map_lookup_elem(std::unordered_map<void*, void*> map, void *key) {
    auto it = map.find(key);
    if (it != map.end()) {
        return it->second;
    }
    return nullptr;
}

long bpf_map_update_elem(std::unordered_map<void*, void*> map, void *key, const void *value, uint64_t flags) {
    map[key] = const_cast<void *>(value);
    return 0;
}

long bpf_map_delete_elem(std::unordered_map<void*, void*> map, void *key) {
    map.erase(key);
    return 0;
}
