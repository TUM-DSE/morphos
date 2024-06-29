#ifndef CLICK_BPFREWRITER_HH
#define CLICK_BPFREWRITER_HH

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

BPFRewriter([I<keywords> PROGRAM])

=s basicsources

Rewrite packets based on an ebpf program.

=d

This element rewrites packets based on an ebpf program.
The BPF program operates on the packet data and can modify it. The program is loaded from a file.

Following additional BPF Helpers are available:
- ID 100: `bpf_packet_add_space(int32_t head_len, int32_t tail_len)`: Adds or removes space to the packet head and tail.

Keyword arguments are:

=over 8

=item FILE

String. Required. File name of the ebpf program defining the rewriter.

 */
class BPFRewriter : public BPFElement {
public:

    BPFRewriter() CLICK_COLD;

    const char *class_name() const override { return "BPFRewriter"; }

    const char *port_count() const override { return PORTS_1_1; }

    void push(int, Packet *) override;

protected:

    virtual void register_additional_bpf_helpers(void) override;
};

CLICK_ENDDECLS
#endif