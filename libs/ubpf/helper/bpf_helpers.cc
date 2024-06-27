#include "bpf_helpers.hh"

#include <cstdarg>
#include <cstdio>
#include <cstdint>
#include <cstdlib>
#include <chrono>
#include <ctime>
#include <random>

void *bpf_map_lookup_elem(void *raw_map, void *key) {
    printf("bpf_map_lookup_elem\n");

    bpf_map &map = *reinterpret_cast<bpf_map *>(raw_map);
    switch (map.def.type) {
        case BPF_MAP_TYPE_HASH: {
            auto *hash_map = static_cast<std::unordered_map <KeyType, ValueType, VectorHash, VectorEqual> *>(map.data);
            KeyType key_value(map.def.key_size);
            std::memcpy(key_value.data(), key, map.def.key_size);

            auto it = hash_map->find(key_value);
            if (it == hash_map->end()) {
                return nullptr;
            }
            return it->second.data();
        }
        case BPF_MAP_TYPE_ARRAY: {
            auto index = *(uint32_t *) key;
            char *data = static_cast<char *>(map.data);
            return &data[index * map.def.value_size];
        }
        default: {
            fprintf(stderr, "bpf_map_lookup_elem: unsupported map type %d\n", map.def.type);
            return nullptr;
        }
    }
}

long bpf_map_update_elem(void *raw_map, void *key, const void *value, uint64_t flags) {
    printf("bpf_map_update_elem\n");

    bpf_map &map = *reinterpret_cast<bpf_map *>(raw_map);
    switch (map.def.type) {
        case BPF_MAP_TYPE_HASH: {
            auto *hash_map = static_cast<std::unordered_map <KeyType, ValueType, VectorHash, VectorEqual> *>(map.data);

            KeyType key_value(map.def.key_size);
            std::memcpy(key_value.data(), key, map.def.key_size);

            ValueType value_value(map.def.value_size);
            std::memcpy(value_value.data(), value, map.def.value_size);

            (*hash_map)[key_value] = value_value;
            break;
        }
        case BPF_MAP_TYPE_ARRAY: {
            auto index = *(uint32_t *) key;
            char *data = static_cast<char *>(map.data);
            void *value_position = &data[index * map.def.value_size];
            std::memcpy(value_position, value, map.def.value_size);
            break;
        }
        default: {
            fprintf(stderr, "bpf_map_update_elem: unsupported map type %d\n", map.def.type);
            return 0;
        }
    }

    return 0;
}

long bpf_map_delete_elem(void *raw_map, void *key) {
    printf("bpf_map_delete_elem\n");

    bpf_map &map = *reinterpret_cast<bpf_map *>(raw_map);
    switch (map.def.type) {
        case BPF_MAP_TYPE_HASH: {
            auto *hash_map = static_cast<std::unordered_map <KeyType, ValueType, VectorHash, VectorEqual> *>(map.data);
            KeyType key_value(map.def.key_size);
            std::memcpy(key_value.data(), key, map.def.key_size);
            hash_map->erase(key_value);
            return 0;
        }
        default: {
            fprintf(stderr, "bpf_map_delete_elem: unsupported map type %d\n", map.def.type);
            return 0;
        }
    }
}

void bpf_trace_printk(const char *fmt, int fmt_size, ...) {
    va_list args;
    va_start(args, fmt_size);

    printf("bpf_trace_printk: ");
    vprintf(fmt, args);

    va_end(args);
}

uint64_t bpf_ktime_get_ns() {
    struct timespec ts;
    clock_gettime(CLOCK_MONOTONIC, &ts);
    return static_cast<uint64_t>(ts.tv_sec) * 1000000000ull + ts.tv_nsec;
}

uint32_t bpf_get_prandom_u32() {
    std::random_device rd;
    std::mt19937 generator(rd());

    std::uniform_int_distribution <uint32_t> distribution(0, UINT32_MAX);

    return distribution(generator);
}

uint64_t unwind(uint64_t i) {
    return i;
}

uint64_t do_data_relocation(
        void *user_context,
        const uint8_t *data,
        uint64_t data_size,
        uint64_t imm
) {
    auto *ctx = static_cast<bpf_map_ctx *>(user_context);

    auto data_ptr = reinterpret_cast<uint64_t>(data);
    void *mem;
    if (ctx->global_data.find(data_ptr) != ctx->global_data.end()) {
        mem = ctx->global_data[data_ptr];
    } else {
        mem = calloc(data_size, sizeof(uint8_t));
        memcpy(mem, data, data_size);
        ctx->global_data[data_ptr] = mem;
    }

    return reinterpret_cast<uint64_t>(mem) + imm;
}

uint64_t do_map_relocation(
        void *user_context,
        const uint8_t *map_data,
        uint64_t map_data_size,
        const char *symbol_name,
        uint64_t symbol_offset,
        uint64_t symbol_size,
        uint64_t imm
) {
    // hack to support .rodata section
    if (symbol_size == 0) {
        return do_data_relocation(user_context, map_data, map_data_size, imm);
    }

    auto *ctx = static_cast<bpf_map_ctx *>(user_context);
    auto map_definition = *reinterpret_cast<const bpf_map_def *>(map_data + symbol_offset);

    (void) symbol_offset; // unused
    (void) map_data_size; // unused

    if (symbol_size < sizeof(struct bpf_map_def)) {
        fprintf(stderr, "Invalid map size: %d\n", (int) symbol_size);
        return 0;
    }

    auto it = ctx->map_by_name.find(symbol_name);
    if (it != ctx->map_by_name.end()) {
        // check if the map definition is the same
        if (it->second->def.type != map_definition.type ||
            it->second->def.key_size != map_definition.key_size ||
            it->second->def.value_size != map_definition.value_size ||
            it->second->def.max_entries != map_definition.max_entries) {
            fprintf(stderr, "Map %s already exists with different definition\n", symbol_name);
            return 0;
        }

        return reinterpret_cast<uint64_t>(it->second);
    }

    void *data = nullptr;
    switch (map_definition.type) {
        case BPF_MAP_TYPE_HASH: {
            auto *hash_map = new std::unordered_map<KeyType, ValueType, VectorHash, VectorEqual>();
            data = reinterpret_cast<void *>(hash_map);
            break;
        }
        case BPF_MAP_TYPE_ARRAY: {
            if (map_definition.key_size != sizeof(uint32_t)) {
                fprintf(stderr, "Unsupported key size %d\n", map_definition.key_size);
                return 0;
            }

            data = std::calloc(map_definition.max_entries, map_definition.value_size);
            break;
        }
        default: {
            fprintf(stderr, "Unsupported map type %d\n", map_definition.type);
            return 0;
        }
    }

    auto *map = new bpf_map();
    map->def = map_definition;
    map->data = data;

    ctx->map_by_name[symbol_name] = map;

    return reinterpret_cast<uint64_t>(map);
}
