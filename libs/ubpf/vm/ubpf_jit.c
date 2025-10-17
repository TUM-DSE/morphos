// Copyright (c) 2015 Big Switch Networks, Inc
// SPDX-License-Identifier: Apache-2.0

/*
 * Copyright 2015 Big Switch Networks, Inc
 * Copyright 2017 Google Inc.
 * Copyright 2022 Linaro Limited
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

#include "ubpf.h"
#define _GNU_SOURCE
#include <stdio.h>
#include <stdlib.h>
#include <stdbool.h>
#include <string.h>
#include <unistd.h>
#include <sys/mman.h>
#include <errno.h>
#include "ubpf_int.h"
#include <uk/pku.h>
#include <uk/plat/paging.h>
#include "../../click/unikraft/mpkey_allocation.hh"

int
ubpf_translate_ex(struct ubpf_vm* vm, uint8_t* buffer, size_t* size, char** errmsg, enum JitMode jit_mode)
{
    struct ubpf_jit_result jit_result = vm->jit_translate(vm, buffer, size, jit_mode);
    vm->jitted_result = jit_result;
    if (jit_result.errmsg) {
        *errmsg = jit_result.errmsg;
    }
    return jit_result.compile_result == UBPF_JIT_COMPILE_SUCCESS ? 0 : -1;
}

int
ubpf_translate(struct ubpf_vm* vm, uint8_t* buffer, size_t* size, char** errmsg)
{
    return ubpf_translate_ex(vm, buffer, size, errmsg, BasicJitMode);
}

struct ubpf_jit_result
ubpf_translate_null(struct ubpf_vm* vm, uint8_t* buffer, size_t* size, enum JitMode jit_mode)
{
    struct ubpf_jit_result compile_result;
    compile_result.compile_result = UBPF_JIT_COMPILE_FAILURE;
    compile_result.external_dispatcher_offset = 0;

    /* NULL JIT target - just returns an error. */
    UNUSED_PARAMETER(vm);
    UNUSED_PARAMETER(buffer);
    UNUSED_PARAMETER(size);
    UNUSED_PARAMETER(jit_mode);
    compile_result.errmsg = ubpf_error("Code can not be JITed on this target.");
    return compile_result;
}

bool
ubpf_jit_update_dispatcher_null(
    struct ubpf_vm* vm, external_function_dispatcher_t new_dispatcher, uint8_t* buffer, size_t size, uint32_t offset)
{
    UNUSED_PARAMETER(vm);
    UNUSED_PARAMETER(new_dispatcher);
    UNUSED_PARAMETER(buffer);
    UNUSED_PARAMETER(size);
    UNUSED_PARAMETER(offset);
    return false;
}

bool
ubpf_jit_update_helper_null(
    struct ubpf_vm* vm, ext_func new_helper, unsigned int idx, uint8_t* buffer, size_t size, uint32_t offset)
{
    UNUSED_PARAMETER(vm);
    UNUSED_PARAMETER(new_helper);
    UNUSED_PARAMETER(idx);
    UNUSED_PARAMETER(buffer);
    UNUSED_PARAMETER(size);
    UNUSED_PARAMETER(offset);
    return false;
}

int
ubpf_set_jit_code_size(struct ubpf_vm* vm, size_t code_size)
{
    vm->jitter_buffer_size = code_size;
    return 0;
}

ubpf_jit_fn
ubpf_compile(struct ubpf_vm* vm, char** errmsg)
{
    return (ubpf_jit_fn)ubpf_compile_ex(vm, errmsg, BasicJitMode);
}

ubpf_jit_ex_fn
ubpf_compile_ex(struct ubpf_vm* vm, char** errmsg, enum JitMode mode)
{
    void* jitted = NULL;
    uint8_t* buffer = NULL;
    size_t jitted_size;

    if (vm->jitted && vm->jitted_result.compile_result == UBPF_JIT_COMPILE_SUCCESS &&
        vm->jitted_result.jit_mode == mode) {
        return vm->jitted;
    }

    if (vm->jitted) {
        struct uk_pagetable *pt = ukplat_pt_get_active();
        int pages = (vm->jitted_size / __PAGE_SIZE) + 1;
		/* uk_pr_err("Unmap %p, %d\n", vm->jitted, pages); */
        ukplat_page_unmap(pt, vm->jitted, pages, 0);
        vm->jitted = NULL;
        vm->jitted_size = 0;
   }

    *errmsg = NULL;

    if (!vm->insts) {
        *errmsg = ubpf_error("code has not been loaded into this VM");
        return NULL;
    }

    jitted_size = vm->jitter_buffer_size;
    buffer = calloc(jitted_size, 1);
    if (buffer == NULL) {
        *errmsg = ubpf_error("internal uBPF error: calloc failed: %s\n", strerror(errno));
        goto out;
    }

    if (ubpf_translate_ex(vm, buffer, &jitted_size, errmsg, mode) < 0) {
        goto out;
    }

    /* jitted = mmap(0, jitted_size, PROT_READ | PROT_WRITE | PROT_EXEC, MAP_PRIVATE | MAP_ANONYMOUS, -1, 0); */
    int pages = (jitted_size / __PAGE_SIZE) + 1;
    struct uk_pagetable *pt = ukplat_pt_get_active();
    jitted = (void*)0x80000000 + 2*__PAGE_SIZE; // the second page is used for _ubpf_jit_stack_protector
    int rc = ukplat_page_map(pt, jitted,
		     __PADDR_ANY, pages,
		     PAGE_ATTR_PROT_READ | PAGE_ATTR_PROT_WRITE, 0);
	if (rc) {
        *errmsg = ubpf_error("can't allocate memory for JIT code: %s\n", strerror(errno));
        jitted = NULL;
        goto out;
	}

    if (jitted == MAP_FAILED) {
        *errmsg = ubpf_error("internal uBPF error: mmap failed: %s\n", strerror(errno));
        jitted = NULL;
        goto out;
    }

    memcpy(jitted, buffer, jitted_size);

    // revoke write permissions (current pkey_mprotect impl does the same again)
    rc = ukplat_page_set_attr(pt, jitted,
			 pages, PAGE_ATTR_PROT_READ | PAGE_ATTR_PROT_EXEC, 0);
    if (rc) {
        *errmsg = ubpf_error("internal uBPF error: mprotect failed: %s\n", strerror(errno));
        jitted = NULL;
        goto out;
    }
    /* ubpf JIT VM not only executes, but also reads JITed code (to find a
     * potential TARGET_PC_EXTERNAL_DISPATCHER for ebpf helper functions).
     * Add MPKEY_STACK to allow reads also in eBPF context with MPK.
     */
#ifdef CONFIG_LIBUBPF_ENABLE_MPK
	  rc = pkey_mprotect(jitted, pages*__PAGE_SIZE, PAGE_ATTR_PROT_READ | PAGE_ATTR_PROT_EXEC, MPKEY_STACK);
	  if (rc < 0) {
		    uk_pr_err("Could not set pkey for ebpf stack %d\n", errno);
		    return -1;
	  }
#endif

    vm->jitted = jitted;
    vm->jitted_size = jitted_size;

out:
    free(buffer);
    if (jitted && vm->jitted == NULL) {
        munmap(jitted, jitted_size);
    }
    return vm->jitted;
}

ubpf_jit_fn
ubpf_copy_jit(struct ubpf_vm* vm, void* buffer, size_t size, char** errmsg)
{
    // If compilation was not successfull or it has not even been attempted,
    // we cannot copy.
    if (vm->jitted_result.compile_result != UBPF_JIT_COMPILE_SUCCESS || !vm->jitted) {
        *errmsg = ubpf_error("Cannot copy JIT'd code before compilation");
        return (ubpf_jit_fn)NULL;
    }

    // If the given buffer is not big enough to contain the JIT'd code,
    // we cannot copy.
    if (vm->jitted_size > size) {
        *errmsg = ubpf_error("Buffer not big enough for copy");
        return (ubpf_jit_fn)NULL;
    }

    // All good. Do the copy!
    memcpy(buffer, vm->jitted, vm->jitted_size);
    *errmsg = NULL;
    return (ubpf_jit_fn)buffer;
}
