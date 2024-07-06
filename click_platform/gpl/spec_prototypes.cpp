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


static const struct EbpfHelperPrototype bpf_packet_add_space_proto = {
        .name = "packet_add_space",
        .return_type = EBPF_RETURN_TYPE_INTEGER,
        .argument_type = {
                EBPF_ARGUMENT_TYPE_ANYTHING,
                EBPF_ARGUMENT_TYPE_ANYTHING,
        },
        .reallocate_packet = true,
}

#define FN(x) bpf_##x##_proto
// keep this on a round line
const struct EbpfHelperPrototype prototypes[] = {
        FN(map_lookup_elem),
        FN(map_update_elem),
        FN(map_delete_elem),
        FN(ktime_get_ns),
        FN(trace_printk),
        FN(get_prandom_u32),
        FN(packet_add_space),
};

EbpfHelperPrototype get_helper_prototype_unchecked(int32_t n) {
    switch (n) {
        case 1:
            return FN(map_lookup_elem);
        case 2:
            return FN(map_update_elem);
        case 3:
            return FN(map_delete_elem);
        case 5:
            return FN(ktime_get_ns);
        case 6:
            return FN(trace_printk);
        case 7:
            return FN(get_prandom_u32);
        case 60:
            return FN(packet_add_space);
        default:
            throw std::exception();
    }
}

bool is_helper_usable_linux(int32_t n) {
    EbpfHelperPrototype prototype = get_helper_prototype_unchecked(n);

    // If the helper has a context_descriptor, it must match the hook's context_descriptor.
    if ((prototype.context_descriptor != nullptr) &&
        (prototype.context_descriptor != global_program_info->type.context_descriptor))
        return false;

    return true;
}

EbpfHelperPrototype get_helper_prototype_linux(int32_t n) {
    if (!is_helper_usable_linux(n))
        throw std::exception();

    return get_helper_prototype_unchecked(n);
}
