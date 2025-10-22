#include <click/config.h>
#include "delay.hh"
#include <click/glue.hh>
#include <click/args.hh>
#include <click/error.hh>
#include <click/straccum.hh>
#ifdef CLICK_LINUXMODULE
# include <click/cxxprotect.h>
CLICK_CXX_PROTECT
# include <linux/sched.h>
CLICK_CXX_UNPROTECT
# include <click/cxxunprotect.h>
#endif
#ifdef CONFIG_LIBCLICK
#include <uk/plat/time.h>
#endif
CLICK_DECLS

Delay::Delay()
{
}

int
Delay::configure(Vector<String> &conf, ErrorHandler* errh)
{
  bool timestamp = false;
#ifdef CLICK_LINUXMODULE
  bool print_cpu = false;
#endif
  bool print_anno = false, headroom = false, bcontents;
  _active = true;
  String label, contents = "HEX";
  int bytes = 24;

    if (Args(conf, this, errh)
	.read_p("LABEL", label)
	.read_p("MAXLENGTH", bytes)
	.read("LENGTH", Args::deprecated, bytes)
	.read("NBYTES", Args::deprecated, bytes)
	.read("CONTENTS", WordArg(), contents)
	.read("TIMESTAMP", timestamp)
	.read("PRINTANNO", print_anno)
	.read("ACTIVE", _active)
	.read("HEADROOM", headroom)
#if CLICK_LINUXMODULE
	.read("CPU", print_cpu)
#endif
	.complete() < 0)
	return -1;

    if (BoolArg().parse(contents, bcontents))
      _contents = bcontents;
  else if ((contents = contents.upper()), contents == "NONE")
      _contents = 0;
  else if (contents == "HEX")
      _contents = 1;
  else if (contents == "ASCII")
      _contents = 2;
  else
      return errh->error("bad contents value '%s'; should be 'NONE', 'HEX', or 'ASCII'", contents.c_str());

  _label = label;
  _bytes = bytes;
  _timestamp = timestamp;
  _headroom = headroom;
  _print_anno = print_anno;
#ifdef CLICK_LINUXMODULE
  _cpu = print_cpu;
#endif
  return 0;
}

Packet *
Delay::simple_action(Packet *p)
{
  auto start = ukplat_monotonic_clock();
  while (true) {
    // 16 nops for good measure
    __asm__("nop\n\t");
    __asm__("nop\n\t");
    __asm__("nop\n\t");
    __asm__("nop\n\t");
    __asm__("nop\n\t");
    __asm__("nop\n\t");
    __asm__("nop\n\t");
    __asm__("nop\n\t");
    __asm__("nop\n\t");
    __asm__("nop\n\t");
    __asm__("nop\n\t");
    __asm__("nop\n\t");
    __asm__("nop\n\t");
    __asm__("nop\n\t");
    __asm__("nop\n\t");
    __asm__("nop\n\t");
    // we use _bytes as delay in nsec
    if ((ukplat_monotonic_clock() - start) >= _bytes) {
      break;
    }
  }

  return p;
}

void
Delay::add_handlers()
{
    add_data_handlers("active", Handler::OP_READ | Handler::OP_WRITE | Handler::CHECKBOX | Handler::CALM, &_active);
}

CLICK_ENDDECLS
EXPORT_ELEMENT(Delay)
ELEMENT_MT_SAFE(Delay)
