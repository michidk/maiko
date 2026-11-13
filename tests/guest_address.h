#ifndef MAIKO_TEST_GUEST_ADDRESS_H
#define MAIKO_TEST_GUEST_ADDRESS_H
#include "guest_abi.h"
#include <assert.h>

/* Test pointers may be native low addresses or offsets into SR's arena. */
static inline uint32_t rg_test_address(const void *pointer) {
    if (!pointer) return 0;
    uint32_t address = 0;
    assert(rg_guest_pointer(pointer, 0, &address));
    return address;
}
#define RG_ADDRESS(pointer) rg_test_address((const void *)(uintptr_t)(pointer))
#endif
