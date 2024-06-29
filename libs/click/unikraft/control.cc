/*
 * control.{cc,hh} -- element which processes control packets
 */

#include <click/config.h>
#include <click/confparse.hh>
#include "control.hh"
#include <click/standard/scheduleinfo.hh>
#include <click/router.hh>
#include <clicknet/udp.h>
#include "bpfelement.hh"

#include <uk/print.h>

CLICK_DECLS

Control::Control() {
}

void Control::push(int, Packet *p) {
    p->kill();

    const unsigned char *udp_data_ptr = p->transport_header() + sizeof(struct click_udp);

    // control packet format:
    // - "control"
    // - uint64_t bpfelement_id
    // - uint64_t program_name_len
    // - char[program_name_len] program_name

    uint64_t offset = 0;

    // check for "control" prefix
    if (p->udp_header()->uh_ulen < 7 + sizeof(uint64_t) + sizeof(uint64_t)) {
        uk_pr_err("Received control packet with invalid length\n");
        return;
    }

    if (memcmp(udp_data_ptr, "control", 7)) {
        uk_pr_err("Received control packet with not-matching prefix\n");
        return;
    }

    offset += 7;

    // parse bpfelement_id
    uint64_t bpfelement_id = *(uint64_t * )(udp_data_ptr + offset);
    offset += sizeof(uint64_t);

    // parse program_name_len
    uint64_t program_name_len = *(uint64_t * )(udp_data_ptr + offset);
    offset += sizeof(uint64_t);

    if (udp_data_ptr + offset + program_name_len > p->end_data()) {
        uk_pr_err("Received control packet with invalid program_name_len\n");
        return;
    }

    // parse program_name
    String program_name((const char *) (udp_data_ptr + offset), program_name_len);

    uk_pr_info("Received control packet for bpfelement_id %lu with program_name %s \n", bpfelement_id,
               program_name.c_str());

    for (int i = 0; i < router()->nelements(); i++) {
        Element *element = router()->element(i);
        if (strcmp(element->class_name(), "BPFilter")
            && strcmp(element->class_name(), "BPFClassifier")
            && strcmp(element->class_name(), "BPFRewriter")) {
            continue;
        }

        BPFElement *bpfelement = static_cast<BPFElement *>(element);
        if (bpfelement->bpfelement_id() != bpfelement_id) {
            continue;
        }

        const Handler *h = Router::handler(element, "config");
        if (!h || !h->write_visible() || !h->writable()) {
            uk_pr_err("Control: %s found but no config handler\n", element->class_name());
            continue;
        }

        uk_pr_info("Control: %s with ID %lu found - calling config handler\n", element->class_name(), bpfelement_id);

        char *config;
        asprintf(&config, "ID %lu, FILE %s", bpfelement_id, program_name.c_str());

        h->call_write(config, element, ErrorHandler::default_handler());
        free(config);
    }
}

CLICK_ENDDECLS
EXPORT_ELEMENT(Control)
