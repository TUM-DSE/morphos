/*
 * bpfilter.{cc,hh} -- element which filters packets based on an ebpf program.
 */

#include <click/config.h>
#include <click/confparse.hh>
#include <click/error.hh>
#include <click/standard/scheduleinfo.hh>
#include <stdio.h>
#include <stdlib.h>

#include "bpfilter.hh"

CLICK_DECLS

BPFilter::BPFilter() {
}

void BPFilter::push(int, Packet *p) {
    _count++;

    uk_pr_debug("BPFilter: Received packet\n");

    uk_rwlock_rlock(&_lock);
    uint64_t ret = this->exec(p);
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
