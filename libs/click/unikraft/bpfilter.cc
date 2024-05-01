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

void listdir(const char *name, int indent)
{
    DIR *dir;
    struct dirent *entry;

    if (!(dir = opendir(name)))
        return;

    while ((entry = readdir(dir)) != NULL) {
        if (entry->d_type == DT_DIR) {
            char path[1024];
            if (strcmp(entry->d_name, ".") == 0 || strcmp(entry->d_name, "..") == 0)
                continue;
            snprintf(path, sizeof(path), "%s/%s", name, entry->d_name);
            fprintf(stderr, "%*s[%s]\n", indent, "", entry->d_name);
            listdir(path, indent + 2);
        } else {
            printf("%*s- %s\n", indent, "", entry->d_name);
        }
    }
    closedir(dir);
}

int BPFilter::configure(Vector<String> &conf, ErrorHandler *errh)
{
    if (conf.empty()) {
        return -1;
    }

    _ubpf_vm = ubpf_create();
    if (_ubpf_vm == NULL) {
        fprintf(stderr, "Unable to create ubpf vm\n");
        return -1;
    }

    auto& _program = conf[0];
    const char* filename = _program.c_str();

    fprintf(stderr, "dir:\n");
    listdir("/", 0);
    FILE* file = fopen(filename, "rb");
    if (!file) {
        fprintf(stderr, "Unable to open file (%s): %s\n", filename, strerror(errno));
        return -1;
    }

    fseek(file, 0, SEEK_END);
    size_t file_size = (size_t) ftell(file);
    fseek(file, 0, SEEK_SET);

    unsigned char* buffer = (unsigned char*)malloc(file_size);
    if (!buffer) {
        fclose(file);
        fprintf(stderr, "Error allocating memory for file (%s): %s\n", filename, strerror(errno));
        return -1;
    }

    size_t bytes_read = fread(buffer, 1, file_size, file);
    if (bytes_read != file_size) {
        fclose(file);
        free(buffer);
        fprintf(stderr, "Error reading file (%s): %s\n", filename, strerror(errno));
        return -1;
    }

    printf("Read following bytes from file: %s\n", filename);
    for (int i = 0; i < bytes_read; ++i) {
        printf("%02x", buffer[i]);
    }
    printf("\n");

    fclose(file);

    char* error_msg;
    ubpf_load_elf(_ubpf_vm, buffer, file_size, &error_msg);

    if (error_msg != NULL) {
        fprintf(stderr, "Error loading ubpf program: %s\n", error_msg);
        free(error_msg);
        return -1;
    }

    return 0;
}

void BPFilter::push(int, Packet *p)
{
    _count++;

    uint64_t ret;
    if (ubpf_exec(_ubpf_vm, NULL, 0, &ret) != 0) {
        fprintf(stderr, "Error executing ubpf program\n");
        return;
    }

    if (ret == 1) {
        _filtered++;
        p->kill();
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
    add_data_handlers("count", Handler::OP_READ, &_count);
    add_data_handlers("filtered", Handler::OP_READ, &_filtered);

    add_write_handler("reset_count", write_handler, 0, Handler::h_button | Handler::h_nonexclusive);
}

CLICK_ENDDECLS
ELEMENT_REQUIRES(int64)
EXPORT_ELEMENT(BPFilter)
