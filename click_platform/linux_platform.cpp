// Copyright (c) Prevail Verifier contributors.
// SPDX-License-Identifier: MIT
#include <stdexcept>
#if __linux__
#include <linux/bpf.h>
#define PTYPE(name, descr, native_type, prefixes) \
             {name, descr, native_type, prefixes}
#define PTYPE_PRIVILEGED(name, descr, native_type, prefixes) \
                        {name, descr, native_type, prefixes, true}
#else
#define PTYPE(name, descr, native_type, prefixes) \
             {name, descr, 0, prefixes}
#define PTYPE_PRIVILEGED(name, descr, native_type, prefixes) \
                        {name, descr, 0, prefixes, true}
#endif
#include "crab_verifier.hpp"
#include "helpers.hpp"
#include "platform.hpp"
#include "linux_platform.hpp"
#include "linux/gpl/spec_type_descriptors.hpp"

// Map definitions as they appear in an ELF file, so field width matters.
struct bpf_load_map_def {
    uint32_t type;
    uint32_t key_size;
    uint32_t value_size;
    uint32_t max_entries;
    uint32_t map_flags;
    uint32_t inner_map_idx;
    uint32_t numa_node;
};

static int create_map_linux(uint32_t map_type, uint32_t key_size, uint32_t value_size, uint32_t max_entries,
                            ebpf_verifier_options_t options);

// Allow for comma as a separator between multiple prefixes, to make
// the preprocessor treat a prefix list as one macro argument.
#define COMMA ,

const EbpfProgramType click_bpffilter_program_type =
    PTYPE("click_bpffilter", &bpfilter_descr, BPF_PROG_TYPE_XDP, {"bpffilter"});

const std::vector<EbpfProgramType> program_types = {
    click_bpffilter_program_type,
};

static EbpfProgramType get_program_type_linux(const std::string& section, const std::string& path) {
    EbpfProgramType type{};

    for (const EbpfProgramType& t : program_types) {
        for (const std::string& prefix : t.section_prefixes) {
            if (section.find(prefix) == 0)
                return t;
        }
    }

    return click_bpffilter_program_type;
}

#ifdef __linux__
#define BPF_MAP_TYPE(x) BPF_MAP_TYPE_##x, #x
#else
#define BPF_MAP_TYPE(x) 0, #x
#endif

static const EbpfMapType linux_map_types[] = {
    {BPF_MAP_TYPE(HASH)},
    {BPF_MAP_TYPE(ARRAY), true},
};

EbpfMapType get_map_type_linux(uint32_t platform_specific_type)
{
    uint32_t index = platform_specific_type;
    if ((index == 0) || (index >= sizeof(linux_map_types) / sizeof(linux_map_types[0]))) {
        return linux_map_types[0];
    }
    EbpfMapType type = linux_map_types[index];
#ifdef __linux__
    assert(type.platform_specific_type == platform_specific_type);
#else
    type.platform_specific_type = platform_specific_type;
#endif
    return type;
}

void parse_maps_section_linux(std::vector<EbpfMapDescriptor>& map_descriptors, const char* data, size_t map_def_size,
                              int map_count, const ebpf_platform_t* platform, ebpf_verifier_options_t options) {
    // Copy map definitions from the ELF section into a local list.
    auto mapdefs = std::vector<bpf_load_map_def>();
    for (int i = 0; i < map_count; i++) {
        bpf_load_map_def def = {0};
        memcpy(&def, data + i * map_def_size, std::min(map_def_size, sizeof(def)));
        mapdefs.emplace_back(def);
    }

    // Add map definitions into the map_descriptors list.
    for (auto const& s : mapdefs) {
        EbpfMapType type = get_map_type_linux(s.type);
        map_descriptors.emplace_back(EbpfMapDescriptor{
            .original_fd = create_map_linux(s.type, s.key_size, s.value_size, s.max_entries, options),
            .type = s.type,
            .key_size = s.key_size,
            .value_size = s.value_size,
            .max_entries = s.max_entries,
            .inner_map_fd = s.inner_map_idx // Temporarily fill in the index. This will be replaced in the
                                            // resolve_inner_map_references pass.
        });
    }
}

// Initialize the inner_map_fd in each map descriptor.
void resolve_inner_map_references_linux(std::vector<EbpfMapDescriptor>& map_descriptors) {
    for (size_t i = 0; i < map_descriptors.size(); i++) {
        unsigned int inner = map_descriptors[i].inner_map_fd; // Get the inner_map_idx back.
        if (inner >= map_descriptors.size())
            throw std::runtime_error(std::string("bad inner map index ") + std::to_string(inner)
                                     + " for map " + std::to_string(i));
        map_descriptors[i].inner_map_fd = map_descriptors.at(inner).original_fd;
    }
}

#if __linux__
static int do_bpf(bpf_cmd cmd, union bpf_attr& attr) { return syscall(321, cmd, &attr, sizeof(attr)); }
#endif

/** Try to allocate a Linux map.
 *
 *  This function requires admin privileges.
 */
static int create_map_linux(uint32_t map_type, uint32_t key_size, uint32_t value_size, uint32_t max_entries,
                            ebpf_verifier_options_t options)
{
    if (options.mock_map_fds) {
        EbpfMapType type = get_map_type_linux(map_type);
        return create_map_crab(type, key_size, value_size, max_entries, options);
    }

#if __linux__
    union bpf_attr attr {};
    memset(&attr, '\0', sizeof(attr));
    attr.map_type = map_type;
    attr.key_size = key_size;
    attr.value_size = value_size;
    attr.max_entries = 20;
    attr.map_flags = map_type == BPF_MAP_TYPE_HASH ? BPF_F_NO_PREALLOC : 0;
    int map_fd = do_bpf(BPF_MAP_CREATE, attr);
    if (map_fd < 0) {
        if (options.print_failures) {
            std::cerr << "Failed to create map, " << strerror(errno) << "\n";
            std::cerr << "Map: \n"
                      << " map_type = " << attr.map_type << "\n"
                      << " key_size = " << attr.key_size << "\n"
                      << " value_size = " << attr.value_size << "\n"
                      << " max_entries = " << attr.max_entries << "\n"
                      << " map_flags = " << attr.map_flags << "\n";
        }
        exit(2);
    }
    return map_fd;
#else
    throw std::runtime_error(std::string("cannot create a Linux map"));
#endif
}

EbpfMapDescriptor& get_map_descriptor_linux(int map_fd)
{
    // First check if we already have the map descriptor cached.
    EbpfMapDescriptor* map = find_map_descriptor(map_fd);
    if (map != nullptr) {
        return *map;
    }

    // This fd was not created from the maps section of an ELF file,
    // but it may be an fd created by an app before calling the verifier.
    // In this case, we would like to query the map descriptor info
    // (key size, value size) from the execution context, but this is
    // not yet supported on Linux.

    throw std::runtime_error(std::string("map_fd not found"));
}

const ebpf_platform_t g_ebpf_platform_linux = {
    get_program_type_linux,
    get_helper_prototype_linux,
    is_helper_usable_linux,
    sizeof(bpf_load_map_def),
    parse_maps_section_linux,
    get_map_descriptor_linux,
    get_map_type_linux,
    resolve_inner_map_references_linux,
    bpf_conformance_groups_t::default_groups | bpf_conformance_groups_t::packet
};
