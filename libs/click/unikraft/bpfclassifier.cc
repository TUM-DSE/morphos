/*
 * bPFClassifier.{cc,hh} -- element which filters packets based on an ebpf program.
 */

#include <click/config.h>
#include <click/confparse.hh>
#include <click/error.hh>
#include <click/standard/scheduleinfo.hh>
#include <stdio.h>
#include <stdlib.h>

#include "bpfclassifier.hh"

CLICK_DECLS

BPFClassifier::BPFClassifier() {
}

void BPFClassifier::push(int, Packet *p) {
    uk_pr_debug("BPFClassifier: Received packet\n");

    uk_rwlock_rlock(&_lock);
    uint64_t ret = this->exec(p);
    uk_rwlock_runlock(&_lock);

    if (ret == -1) {
        p->kill();
        return;
    }

    output(ret).push(p);
}

CLICK_ENDDECLS
ELEMENT_REQUIRES(int64)

EXPORT_ELEMENT(BPFClassifier)
