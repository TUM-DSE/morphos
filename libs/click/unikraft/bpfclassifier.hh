#ifndef CLICK_BPFCLASSIFIER_HH
#define CLICK_BPFCLASSIFIER_HH

#include <click/config.h>
#include <click/deque.hh>
#include <click/element.hh>
#include <click/error.hh>
#include <click/task.hh>
#include <uk/rwlock.h>
#include <bpf_helpers.hh>
#include <ubpf.h>
#include "bpfelement.hh"

CLICK_DECLS

/*
=c

BPFClassifier([I<keywords> PROGRAM])

=s basicsources

filters packets based on an ebpf program

=d

Classify packets based on an ebpf program. The output port is determined by the return value of the ebpf program.
The ebpf program is loaded from a file.

Keyword arguments are:

=over 8

=item FILE

String. Required. File name of the ebpf program defining the classifier.

 */
class BPFClassifier : public BPFElement {
public:

    BPFClassifier() CLICK_COLD;

    const char *class_name() const override { return "BPFClassifier"; }

    const char *port_count() const override { return "-/-"; }

    const char *processing() const override { return PUSH; }

    void push(int, Packet *) override;

};

CLICK_ENDDECLS
#endif
