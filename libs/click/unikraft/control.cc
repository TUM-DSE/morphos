/*
 * control.{cc,hh} -- element which processes control packets
 */

#include <click/config.h>
#include <click/confparse.hh>
#include "control.hh"
#include <click/standard/scheduleinfo.hh>
#include <click/router.hh>

#include <uk/print.h>

CLICK_DECLS

Control::Control() {
}

void Control::push(int, Packet *p) {
    p->kill();
    uk_pr_info("Control: Received packet\n");

    for (int i = 0; i < router()->nelements(); i++) {
        Element *element = router()->element(i);
        if (!!strcmp(element->class_name(), "BPFilter")) continue;

        const Handler *h = Router::handler(element, "config");
        if (!h || !h->write_visible() || !h->writable()) {
            uk_pr_err("Control: BPFilter found but no config handler\n");
            continue;
        }

        uk_pr_info("Control: BPFilter found - calling config handler\n");
        h->call_write("", element, ErrorHandler::default_handler());
    }
}

CLICK_ENDDECLS
EXPORT_ELEMENT(Control)
