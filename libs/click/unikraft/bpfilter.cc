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

static void ubpf_print(const char *msg) {
    printf("%s", msg);
}

int BPFilter::configure(Vector<String> &conf, ErrorHandler *errh)
{
    if (conf.empty()) {
        return -1;
    }

    uk_pr_info("Configuring BPFilter...\n");

    bool reconfigure = false;
    if (_ubpf_vm == NULL) {
        _ubpf_vm = ubpf_create();
        if (_ubpf_vm == NULL) {
            return errh->error("unable to create ubpf vm\n");
        }

        ubpf_register(_ubpf_vm, 0, "ubpf_print", (void*) ubpf_print);
    } else {
        reconfigure = true;
    }

    String program_string = String();
    if (Args(conf, this, errh)
    .read("ID", _bpfilter_id)
    .read("FILE", AnyArg(), program_string)
    .complete() < 0) {
        return -1;
    }

    const char* filename = program_string.c_str();

    FILE* file = fopen(filename, "rb");
    if (!file) {
        return errh->error("unable to open file (%s): %s\n", filename, strerror(errno));
    }

    fseek(file, 0, SEEK_END);
    size_t file_size = (size_t) ftell(file);
    fseek(file, 0, SEEK_SET);

    unsigned char* buffer = (unsigned char*)malloc(file_size);
    if (!buffer) {
        return errh->error("error allocating memory for file (%s): %s\n", filename, strerror(errno));
    }

    size_t bytes_read = fread(buffer, 1, file_size, file);
    if (bytes_read != file_size) {
        fclose(file);
        free(buffer);
        return errh->error("error reading file (%s): %s\n", filename, strerror(errno));
    }

    fclose(file);

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

    uk_pr_info("Configured BPFilter (ID: %lu) with program %s\n", _bpfilter_id, filename);

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
