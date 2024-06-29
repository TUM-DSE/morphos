#include <click/config.h>
#include <click/confparse.hh>
#include <click/error.hh>
#include <click/standard/scheduleinfo.hh>
#include <stdio.h>
#include <stdlib.h>

#include "bpfrewriter.hh"

CLICK_DECLS

BPFRewriter::BPFRewriter() {
}

void BPFRewriter::register_additional_bpf_helpers(void) {
    // TODO: bpf_skb_adjust_room?
    // ubpf_register(_ubpf_vm, 100, "bpf_packet_add_space", as_external_function_t((void*) bpf_packet_add_space));
}

void BPFRewriter::push(int, Packet *p) {
    uk_pr_debug("BPFRewriter: Received packet\n");

    WritablePacket *p_out = p->uniqueify();
    if (!p_out) {
        return;
    }

    uk_rwlock_rlock(&_lock);
    int ret = this->exec(p_out);
    uk_rwlock_runlock(&_lock);

    if (ret == -1) {
        p_out->kill();
        return;
    }

    output(0).push(p_out);
}

CLICK_ENDDECLS
ELEMENT_REQUIRES(int64)

EXPORT_ELEMENT(BPFRewriter)
