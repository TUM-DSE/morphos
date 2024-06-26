/*
 * bpfilter.{cc,hh} -- element which filters packets based on an ebpf program.
 */

#include <click/config.h>
#include <click/confparse.hh>
#include <click/error.hh>
#include <click/args.hh>
#include <click/standard/scheduleinfo.hh>
#include <stdio.h>
#include <dirent.h>
#include <stdlib.h>

#include "bpfilter.hh"

CLICK_DECLS

BPFilter::BPFilter() {
}

char *read_file(const char *filename, size_t *size) {
    FILE *file = fopen(filename, "rb");
    if (!file) {
        return NULL;
    }

    fseek(file, 0, SEEK_END);
    size_t file_size = (size_t) ftell(file);
    fseek(file, 0, SEEK_SET);

    unsigned char *buffer = (unsigned char *) malloc(file_size);
    if (!buffer) {
        fclose(file);
        return NULL;
    }

    size_t bytes_read = fread(buffer, 1, file_size, file);
    if (bytes_read != file_size) {
        fclose(file);
        free(buffer);
        return NULL;
    }

    fclose(file);

    *size = file_size;
    return (char *) buffer;
}

char write_file(const char *filename, void *buffer, size_t size) {
    FILE *file = fopen(filename, "wb");
    if (!file) {
        return -1;
    }

    size_t bytes_written = fwrite(buffer, 1, size, file);
    if (bytes_written != size) {
        fclose(file);
        return -1;
    }

    fclose(file);
    return 0;
}

ubpf_vm *BPFilter::init_ubpf_vm() {
    ubpf_vm *vm = ubpf_create();
    if (vm == NULL) {
        return NULL;
    }

    this->_bpf_map_ctx = new bpf_map_ctx();

    ubpf_toggle_bounds_check(vm, false);
    ubpf_toggle_undefined_behavior_check(vm, false);
    ubpf_register_data_relocation(vm, this->_bpf_map_ctx, do_map_relocation);

    // register bpf helpers
    ubpf_register(vm, 1, "bpf_map_lookup_elem", as_external_function_t((void *) bpf_map_lookup_elem));
    ubpf_register(vm, 2, "bpf_map_update_elem", as_external_function_t((void *) bpf_map_update_elem));
    ubpf_register(vm, 3, "bpf_map_delete_elem", as_external_function_t((void *) bpf_map_delete_elem));
    ubpf_register(vm, 5, "bpf_ktime_get_ns", as_external_function_t((void *) bpf_ktime_get_ns));
    ubpf_register(vm, 6, "bpf_trace_printk", as_external_function_t((void *) bpf_trace_printk));
    ubpf_register(vm, 7, "bpf_get_prandom_u32", as_external_function_t((void *) bpf_get_prandom_u32));
    ubpf_register(vm, 20, "unwind", as_external_function_t((void *) unwind));
    ubpf_set_unwind_function_index(vm, 20);

    return vm;
}

int BPFilter::configure(Vector <String> &conf, ErrorHandler *errh) {
    if (conf.empty()) {
        return -1;
    }

    bool dump_jit;
    String program_string = String();
    if (Args(conf, this, errh)
                .read("ID", _bpfilter_id)
                .read("JIT", _jit)
                .read("FILE", AnyArg(), program_string)
                .read("DUMP_JIT", dump_jit)
                .complete() < 0) {
        return -1;
    }

    const char *filename = program_string.c_str();

    bool reconfigure = _ubpf_vm != NULL;
    if (reconfigure) {
        uk_pr_info("Reconfiguring BPFilter (ID: %lu - JIT: %d) with program %s..\n", _bpfilter_id, _jit, filename);
    } else {
        uk_pr_info("Configuring BPFilter (ID: %lu - JIT: %d) with program %s...\n", _bpfilter_id, _jit, filename);
    }

    size_t file_size;
    char *buffer = read_file(filename, &file_size);
    if (buffer == NULL) {
        return errh->error("Error reading file %s\n", filename);
    }

    if (!reconfigure) {
        _ubpf_vm = this->init_ubpf_vm();
        if (_ubpf_vm == NULL) {
            return errh->error("Error initializing ubpf vm\n");
        }
    }

    uk_rwlock_wlock(&_lock);
    if (reconfigure) {
        ubpf_unload_code(_ubpf_vm);
    }

    char *error_msg;
    ubpf_load_elf(_ubpf_vm, buffer, file_size, &error_msg);

    if (error_msg != NULL) {
        return errh->error("Error loading ubpf program: %s\n", error_msg);
    }

    if (_jit) {
        _ubpf_jit_fn = ubpf_compile(_ubpf_vm, &error_msg);
        if (_ubpf_jit_fn == NULL) {
            return errh->error("Error compiling ubpf program: %s\n", error_msg);
        }
    }

    if (dump_jit) {
        uint8_t *buffer = (uint8_t *) calloc(65536, 1);
        if (buffer == NULL) {
            return errh->error("Error allocating buffer for jit dump\n");
        }

        size_t jitted_size;
        if (ubpf_translate(_ubpf_vm, buffer, &jitted_size, &error_msg) < 0) {
            return errh->error("Error translating ubpf program: %s\n", error_msg);
        }

        write_file("jit_dump.bin", buffer, jitted_size);
        free(buffer);

        uk_pr_info("Dumped JIT code to jit_dump.bin\n");
    }

    uk_rwlock_wunlock(&_lock);

    if (reconfigure) {
        uk_pr_info("Reconfigured BPFilter (ID: %lu - JIT: %d) with program %s\n", _bpfilter_id, _jit, filename);
    } else {
        uk_pr_info("Configured BPFilter (ID: %lu - JIT: %d) with program %s\n", _bpfilter_id, _jit, filename);
    }

    return 0;
}

inline int BPFilter::exec_filter(Packet *p) {
    if (_jit) {
        return _ubpf_jit_fn((void *) p->buffer(), p->buffer_length());
    } else {
        uint64_t ret;
        if (ubpf_exec(_ubpf_vm, (void *) p->buffer(), p->buffer_length(), &ret) != 0) {
            uk_pr_err("Error executing filter\n");
            return -1;
        }

        return ret;
    }
}

void BPFilter::push(int, Packet *p) {
    _count++;

    uk_pr_debug("BPFilter: Received packet\n");

    uk_rwlock_rlock(&_lock);
    uint64_t ret = exec_filter(p);
    uk_rwlock_runlock(&_lock);

    if (ret == 1) {
        uk_pr_debug("BPFilter: Dropped packet\n");
        _filtered++;
        p->kill();
    } else {
        uk_pr_debug("BPFilter: Didn't drop packet\n");
        output(0).push(p);
    }
}

int
BPFilter::write_handler(const String &s, Element *e, void *user_data,
                        ErrorHandler *errh) {
    BPFilter *bp_filter = static_cast<BPFilter *>(e);
    bp_filter->_count = 0;
    bp_filter->_filtered = 0;

    return 0;
}

void
BPFilter::add_handlers() {
    add_data_handlers("count", Handler::h_read, &_count);
    add_data_handlers("filtered", Handler::h_read, &_filtered);

    add_write_handler("reset_count", write_handler, 0, Handler::h_button | Handler::h_nonexclusive);
}

CLICK_ENDDECLS
ELEMENT_REQUIRES(int64)

EXPORT_ELEMENT(BPFilter)
