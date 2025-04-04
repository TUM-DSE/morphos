extern "C" {
#include <sys/mman.h>
#include <uk/pku.h>
#include <uk/plat/paging.h>
}

#include "mpkey_allocation.hh"

static bool allocated = false;

int mpkey_allocation_alloc() {
  // uk_pr_err("Alloc mpkeys\n");
  if (allocated) {
    // uk_pr_err("MPKEYs already allocated\n");
    return 0;
  }

  // key 0 is allocated by default
  for (int i = 1; i < NUM_MPKEYS; i++) {
	  int pkey = pkey_alloc(0, 0);
	  if (pkey < 0) {
		  uk_pr_err("Could not allocate pkey %d\n", pkey);
		  return -1;
	  }
	  // uk_pr_err("got key %d\n", pkey);
	  UK_ASSERT(pkey == i); // assume that we are the only ones allocating pkeys
  }

  allocated = true;

  return 0;
}
