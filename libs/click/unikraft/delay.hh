#ifndef CLICK_DELAY_HH
#define CLICK_DELAY_HH
#include <click/element.hh>
CLICK_DECLS

class Delay : public Element { public:

    Delay() CLICK_COLD;

    const char *class_name() const		{ return "Delay"; }
    const char *port_count() const		{ return PORTS_1_1; }

    int configure(Vector<String> &, ErrorHandler *) CLICK_COLD;
    bool can_live_reconfigure() const		{ return true; }
    void add_handlers() CLICK_COLD;

    Packet *simple_action(Packet *);

 private:

    String _label;
    int _bytes;		// How many bytes of a packet to print
    bool _active;
    bool _timestamp : 1;
    bool _headroom : 1;
#ifdef CLICK_LINUXMODULE
    bool _cpu : 1;
#endif
    bool _print_anno;
    uint8_t _contents;

};

CLICK_ENDDECLS
#endif

