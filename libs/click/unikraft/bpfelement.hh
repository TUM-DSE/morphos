#ifndef CLICK_BPFELEMENT_HH
#define CLICK_BPFELEMENT_HH

#include <click/config.h>
#include <click/deque.hh>
#include <click/element.hh>
#include <click/error.hh>
#include <click/task.hh>
#include <uk/rwlock.h>
#include <bpf_helpers.hh>
#include <ubpf.h>

CLICK_DECLS

class BPFElement : public Element {
public:

    bool can_live_reconfigure() const override { return true; }

    int configure(Vector <String> &conf, ErrorHandler *errh) override CLICK_COLD;

    uint64_t bpfelement_id() const { return _bpfelement_id; }

protected:

    struct uk_rwlock _lock = UK_RWLOCK_INITIALIZER(_lock, 0);
    struct ubpf_vm *_ubpf_vm;

    virtual void register_additional_bpf_helpers(void) { }

    uint32_t exec(Packet *p);

private:

    uint64_t _bpfelement_id;
    bool _jit;
    bool _dump_jit;
    String _bpf_file;
    String _signature_file;

    struct bpf_map_ctx *_bpf_map_ctx;
    ubpf_jit_fn _ubpf_jit_fn;

    void init_ubpf_vm();
    int check_bpf_verification_signature(ErrorHandler *errh);

    CLICK_COLD;
};

typedef struct {
    void* data;
    void* data_end;
} bpfelement_md;

CLICK_ENDDECLS
#endif