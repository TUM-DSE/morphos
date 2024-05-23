#ifndef CLICK_CONTROL_HH
#define CLICK_CONTROL_HH

#include <click/config.h>
#include <click/deque.hh>
#include <click/element.hh>
#include <click/error.hh>
#include <click/task.hh>

CLICK_DECLS

/*
=c

Control([I<keywords> PROGRAM])

=s basicsources

Element that can be used to trigger live reconfiguration of other elements.

 */
class Control : public Element { public:

    Control() CLICK_COLD;

    const char *class_name() const override		{ return "Control"; }
    const char *port_count() const override		{ return PORTS_1_0; }
    bool can_live_reconfigure() const override   { return true; }

    void push(int, Packet *) override;

};

CLICK_ENDDECLS
#endif