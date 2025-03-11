#include <click/config.h>
#include <click/confparse.hh>
#include <click/standard/scheduleinfo.hh>

#include "bpfclassifier.hh"

CLICK_DECLS

BPFClassifier::BPFClassifier() {
}

void BPFClassifier::push(int port, Packet *p) {
    uk_pr_debug("BPFClassifier: Received packet\n");

    WritablePacket * p_out = p->uniqueify();
    if (!p_out) {
        return;
    }

    uk_rwlock_rlock(&_lock);
    int ret = this->exec(port, p_out);
    uk_rwlock_runlock(&_lock);

    if (ret == -1) {
        uk_pr_debug("BPFClassifier: Classifier aborted\n");
        p->kill();
        return;
    }

    checked_output_push(ret, p_out);
}

CLICK_ENDDECLS
ELEMENT_REQUIRES(int64)

EXPORT_ELEMENT(BPFClassifier)
