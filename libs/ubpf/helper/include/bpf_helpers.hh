#ifndef UBPF_HELPERS_HH
#define UBPF_HELPERS_HH

#include <cstdint>
#include <unordered_map>

void bpf_trace(long num);

void *bpf_map_lookup_elem(std::unordered_map<void *, void *> map, void *key);

long bpf_map_update_elem(std::unordered_map<void *, void *> map, void *key, const void *value, uint64_t flags);

long bpf_map_delete_elem(std::unordered_map<void *, void *> map, void *key);

uint64_t unwind(uint64_t i);

#endif /* UBPF_HELPERS_HH */
