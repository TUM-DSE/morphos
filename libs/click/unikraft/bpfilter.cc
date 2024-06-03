/*
 * bpfilter.{cc,hh} -- element which filters packets based on an ebpf program.
 */

#include <click/config.h>
#include <click/confparse.hh>
#include "bpfilter.hh"
#include <click/error.hh>
#include <click/args.hh>
#include <click/standard/scheduleinfo.hh>
#include <stdio.h>
#include <dirent.h>
#include <stdlib.h>

CLICK_DECLS

BPFilter::BPFilter()
{
}

char* BPFilter::read_file(const char* filename, size_t* size)
{
    FILE* file = fopen(filename, "rb");
    if (!file) {
        return NULL;
    }

    fseek(file, 0, SEEK_END);
    size_t file_size = (size_t) ftell(file);
    fseek(file, 0, SEEK_SET);

    unsigned char* buffer = (unsigned char*)malloc(file_size);
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
    return (char*)buffer;
}

int BPFilter::configure(Vector<String> &conf, ErrorHandler *errh)
{
    if (conf.empty()) {
        return -1;
    }

    String program_string = String();
    if (Args(conf, this, errh)
                .read("ID", _bpfilter_id)
                .read("JIT", _jit)
                .read("FILE", AnyArg(), program_string)
                .complete() < 0) {
        return -1;
    }

    const char* filename = program_string.c_str();

    bool reconfigure = _ubpf_vm != NULL;
    if (reconfigure) {
        uk_pr_info("Reconfiguring BPFilter (ID: %lu - JIT: %b) with program %s..\n", _bpfilter_id, _jit, filename);
    } else {
        uk_pr_info("Configuring BPFilter (ID: %lu - JIT: %b) with program %s...\n", _bpfilter_id, _jit, filename);
    }

    size_t file_size;
    char* buffer = read_file(filename, &file_size);
    if (buffer == NULL) {
        return errh->error("Error reading file %s\n", filename);
    }

    if (!reconfigure) {
        _ubpf_vm = ubpf_create();
        if (_ubpf_vm == NULL) {
            return errh->error("Error creating ubpf vm\n");
        }
    }

    uk_rwlock_wlock(&_lock);
    if (reconfigure) {
        ubpf_unload_code(_ubpf_vm);
    }

    char* error_msg;
    ubpf_load_elf(_ubpf_vm, buffer, file_size, &error_msg);

    uk_rwlock_wunlock(&_lock);

    if (error_msg != NULL) {
        return errh->error("Error loading ubpf program: %s\n", error_msg);
    }

    if (reconfigure) {
        uk_pr_info("Reconfigured BPFilter (ID: %lu - JIT: %b) with program %s\n", _bpfilter_id, _jit, filename);
    } else {
        uk_pr_info("Configured BPFilter (ID: %lu - JIT: %b) with program %s\n", _bpfilter_id, _jit, filename);
    }

    return 0;
}

void BPFilter::push(int, Packet *p)
{
    _count++;

    uk_pr_debug("BPFilter: Received packet\n");

    uk_rwlock_rlock(&_lock);
    uint64_t ret;
    if (ubpf_exec(_ubpf_vm, (void*) p->buffer(), p->buffer_length(), &ret) != 0) {
        uk_pr_err("Error executing ubpf program\n");
        return;
    }
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
                        ErrorHandler *errh)
{
    BPFilter * bp_filter = static_cast<BPFilter *>(e);
    bp_filter->_count = 0;
    bp_filter->_filtered = 0;

    return 0;
}

void
BPFilter::add_handlers()
{
    add_data_handlers("count", Handler::h_read, &_count);
    add_data_handlers("filtered", Handler::h_read, &_filtered);

    add_write_handler("reset_count", write_handler, 0, Handler::h_button | Handler::h_nonexclusive);
}

CLICK_ENDDECLS
ELEMENT_REQUIRES(int64)
EXPORT_ELEMENT(BPFilter)
