// Copyright (c) Prevail Verifier contributors.
// SPDX-License-Identifier: MIT
#pragma once

EbpfHelperPrototype get_helper_prototype_click(int32_t n);
bool is_helper_usable_click(int32_t n);

extern const ebpf_platform_t g_ebpf_platform_click;