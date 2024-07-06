#include "platform.hpp"
#include "spec_type_descriptors.hpp"

#define EBPF_RETURN_TYPE_PTR_TO_SOCK_COMMON_OR_NULL   EBPF_RETURN_TYPE_UNSUPPORTED
#define EBPF_RETURN_TYPE_PTR_TO_SOCKET_OR_NULL        EBPF_RETURN_TYPE_UNSUPPORTED
#define EBPF_RETURN_TYPE_PTR_TO_TCP_SOCKET_OR_NULL    EBPF_RETURN_TYPE_UNSUPPORTED
#define EBPF_RETURN_TYPE_PTR_TO_ALLOC_MEM_OR_NULL     EBPF_RETURN_TYPE_UNSUPPORTED
#define EBPF_RETURN_TYPE_PTR_TO_BTF_ID_OR_NULL        EBPF_RETURN_TYPE_UNSUPPORTED
#define EBPF_RETURN_TYPE_PTR_TO_MEM_OR_BTF_ID_OR_NULL EBPF_RETURN_TYPE_UNSUPPORTED
#define EBPF_RETURN_TYPE_PTR_TO_BTF_ID                EBPF_RETURN_TYPE_UNSUPPORTED
#define EBPF_RETURN_TYPE_PTR_TO_MEM_OR_BTF_ID         EBPF_RETURN_TYPE_UNSUPPORTED

#define EBPF_ARGUMENT_TYPE_PTR_TO_BTF_ID_SOCK_COMMON EBPF_ARGUMENT_TYPE_UNSUPPORTED
#define EBPF_ARGUMENT_TYPE_PTR_TO_SPIN_LOCK          EBPF_ARGUMENT_TYPE_UNSUPPORTED
#define EBPF_ARGUMENT_TYPE_PTR_TO_SOCK_COMMON        EBPF_ARGUMENT_TYPE_UNSUPPORTED
#define EBPF_ARGUMENT_TYPE_PTR_TO_BTF_ID             EBPF_ARGUMENT_TYPE_UNSUPPORTED
#define EBPF_ARGUMENT_TYPE_PTR_TO_BTF_ID_SOCK_COMMON EBPF_ARGUMENT_TYPE_UNSUPPORTED
#define EBPF_ARGUMENT_TYPE_PTR_TO_LONG               EBPF_ARGUMENT_TYPE_UNSUPPORTED
#define EBPF_ARGUMENT_TYPE_PTR_TO_INT                EBPF_ARGUMENT_TYPE_UNSUPPORTED
#define EBPF_ARGUMENT_TYPE_PTR_TO_CONST_STR          EBPF_ARGUMENT_TYPE_UNSUPPORTED
#define EBPF_ARGUMENT_TYPE_PTR_TO_FUNC               EBPF_ARGUMENT_TYPE_UNSUPPORTED
#define EBPF_ARGUMENT_TYPE_PTR_TO_STACK_OR_NULL      EBPF_ARGUMENT_TYPE_UNSUPPORTED
#define EBPF_ARGUMENT_TYPE_CONST_ALLOC_SIZE_OR_ZERO  EBPF_ARGUMENT_TYPE_UNSUPPORTED
#define EBPF_ARGUMENT_TYPE_PTR_TO_ALLOC_MEM          EBPF_ARGUMENT_TYPE_UNSUPPORTED
#define EBPF_ARGUMENT_TYPE_PTR_TO_ALLOC_MEM          EBPF_ARGUMENT_TYPE_UNSUPPORTED
#define EBPF_ARGUMENT_TYPE_PTR_TO_MAP_VALUE_OR_NULL  EBPF_ARGUMENT_TYPE_UNSUPPORTED
#define EBPF_ARGUMENT_TYPE_PTR_TO_TIMER              EBPF_ARGUMENT_TYPE_UNSUPPORTED
#define EBPF_ARGUMENT_TYPE_PTR_TO_PERCPU_BTF_ID      EBPF_ARGUMENT_TYPE_UNSUPPORTED

static const struct EbpfHelperPrototype bpf_map_lookup_elem_proto = {
        .name = "map_lookup_elem",
        .return_type = EBPF_RETURN_TYPE_PTR_TO_MAP_VALUE_OR_NULL,
        .argument_type = {
                EBPF_ARGUMENT_TYPE_PTR_TO_MAP,
                EBPF_ARGUMENT_TYPE_PTR_TO_MAP_KEY,
                EBPF_ARGUMENT_TYPE_DONTCARE,
                EBPF_ARGUMENT_TYPE_DONTCARE,
                EBPF_ARGUMENT_TYPE_DONTCARE,
        },
};

static const struct EbpfHelperPrototype bpf_map_update_elem_proto = {
        .name = "map_update_elem",
        .return_type = EBPF_RETURN_TYPE_INTEGER,
        .argument_type = {
                EBPF_ARGUMENT_TYPE_PTR_TO_MAP,
                EBPF_ARGUMENT_TYPE_PTR_TO_MAP_KEY,
                EBPF_ARGUMENT_TYPE_PTR_TO_MAP_VALUE,
                EBPF_ARGUMENT_TYPE_ANYTHING,
                EBPF_ARGUMENT_TYPE_DONTCARE,
        },
};

static const struct EbpfHelperPrototype bpf_map_delete_elem_proto = {
        .name = "map_delete_elem",
        .return_type = EBPF_RETURN_TYPE_INTEGER,
        .argument_type = {
                EBPF_ARGUMENT_TYPE_PTR_TO_MAP,
                EBPF_ARGUMENT_TYPE_PTR_TO_MAP_KEY,
                EBPF_ARGUMENT_TYPE_DONTCARE,
                EBPF_ARGUMENT_TYPE_DONTCARE,
                EBPF_ARGUMENT_TYPE_DONTCARE,
        },
};

static const struct EbpfHelperPrototype bpf_ktime_get_ns_proto = {
        .name = "ktime_get_ns",
        .return_type = EBPF_RETURN_TYPE_INTEGER,
};

static const struct EbpfHelperPrototype bpf_trace_printk_proto = {
        .name = "trace_printk",
        .return_type = EBPF_RETURN_TYPE_INTEGER,
        .argument_type = {
                EBPF_ARGUMENT_TYPE_PTR_TO_READABLE_MEM,
                EBPF_ARGUMENT_TYPE_CONST_SIZE,
        },
};

static const struct EbpfHelperPrototype bpf_get_prandom_u32_proto = {
        .name = "get_prandom_u32",
        .return_type = EBPF_RETURN_TYPE_INTEGER,
};


#define FN(x) bpf_##x##_proto
// keep this on a round line
const struct EbpfHelperPrototype prototypes[] = {
    FN(map_lookup_elem),
    FN(map_update_elem),
    FN(map_delete_elem),
    FN(ktime_get_ns),
    FN(trace_printk),
    FN(get_prandom_u32),
};

bool is_helper_usable_linux(int32_t n) {
    if (n >= (int)(sizeof(prototypes) / sizeof(prototypes[0])) || n < 0)
        return false;

    // If the helper has a context_descriptor, it must match the hook's context_descriptor.
    if ((prototypes[n].context_descriptor != nullptr) &&
        (prototypes[n].context_descriptor != global_program_info->type.context_descriptor))
        return false;

    return true;
}

EbpfHelperPrototype get_helper_prototype_linux(int32_t n) {
    if (!is_helper_usable_linux(n))
        throw std::exception();
    return prototypes[n];
}
