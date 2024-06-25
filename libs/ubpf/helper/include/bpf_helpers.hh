#ifndef UBPF_HELPERS_HH
#define UBPF_HELPERS_HH

#include <cstdint>
#include <unordered_map>
#include <vector>
#include <string>

void bpf_trace(long num);

void *bpf_map_lookup_elem(void *map, void *key);

long bpf_map_update_elem(void *map, void *key, const void *value, uint64_t flags);

long bpf_map_delete_elem(void *map, void *key);

uint64_t bpf_ktime_get_ns(void);

uint32_t bpf_get_prandom_u32(void);

uint64_t unwind(uint64_t i);

enum bpf_map_type {
    BPF_MAP_TYPE_UNSPEC,
    BPF_MAP_TYPE_HASH,
    BPF_MAP_TYPE_ARRAY,
    BPF_MAP_TYPE_PROG_ARRAY,
    BPF_MAP_TYPE_PERF_EVENT_ARRAY,
    BPF_MAP_TYPE_PERCPU_HASH,
    BPF_MAP_TYPE_PERCPU_ARRAY,
    BPF_MAP_TYPE_STACK_TRACE,
    BPF_MAP_TYPE_CGROUP_ARRAY,
    BPF_MAP_TYPE_LRU_HASH,
    BPF_MAP_TYPE_LRU_PERCPU_HASH,
    BPF_MAP_TYPE_LPM_TRIE,
    BPF_MAP_TYPE_ARRAY_OF_MAPS,
    BPF_MAP_TYPE_HASH_OF_MAPS,
    BPF_MAP_TYPE_DEVMAP,
    BPF_MAP_TYPE_SOCKMAP,
    BPF_MAP_TYPE_CPUMAP,
    BPF_MAP_TYPE_XSKMAP,
    BPF_MAP_TYPE_SOCKHASH,
};

struct bpf_map_def {
    unsigned int type;
    unsigned int key_size;
    unsigned int value_size;
    unsigned int max_entries;
    unsigned int map_flags;
    unsigned int inner_map_idx;
    unsigned int numa_node;
};

struct bpf_map {
    struct bpf_map_def def;
    void *data;
};

struct bpf_map_ctx {
    std::unordered_map<std::string, struct bpf_map *> map_by_name;
};

// Type alias for convenience
using KeyType = std::vector<uint8_t>;
using ValueType = std::vector<uint8_t>;

// Hash function for vector<uint8_t>
struct VectorHash {
    std::size_t operator()(const std::vector<uint8_t>& v) const {
        std::size_t hash = 0;
        for (auto byte : v) {
            hash ^= std::hash<uint8_t>()(byte) + 0x9e3779b9 + (hash << 6) + (hash >> 2);
        }
        return hash;
    }
};

// Equality function for vector<uint8_t>
struct VectorEqual {
    bool operator()(const std::vector<uint8_t>& lhs, const std::vector<uint8_t>& rhs) const {
        return lhs == rhs;
    }
};

uint64_t do_map_relocation(
        void *user_context,
        const uint8_t *map_data,
        uint64_t map_data_size,
        const char *symbol_name,
        uint64_t symbol_offset,
        uint64_t symbol_size);

#endif /* UBPF_HELPERS_HH */
