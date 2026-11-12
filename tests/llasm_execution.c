/* Synthetic code only. Exercise upstream's C/guest ABI with offset pointers. */
#define _GNU_SOURCE
#include "llasm_cpu.h"
#include <assert.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <dlfcn.h>
#include <sys/mman.h>
#include <math.h>
#include <fenv.h>

extern void c_rg_round(_cpu *);
extern void c_rg_store_extended(_cpu *);
extern void c_rg_jecxz(_cpu *);
extern void c_rg_jo(_cpu *);
extern void c_rg_jno(_cpu *);
extern void c_rg_ftst(_cpu *);
extern void c_rg_atan(_cpu *);
extern void c_rg_load_extended(_cpu *);
extern void c_rg_remainder(_cpu *);
extern void c_rg_test(_cpu *);
extern void c_rg_test_edi(_cpu *);
extern void c_rg_test_memory(_cpu *);
extern void c_rg_test_float(_cpu *);
extern void c_rg_flags(_cpu *);
extern void c_rg_signed_divide(_cpu *);
extern void c_rg_unsigned_divide(_cpu *);
extern void c_rg_compare(_cpu *);

extern void c_rg_fsubr_to_1(_cpu *);
extern void c_rg_fsubr_to_2(_cpu *);
extern void c_rg_fsubr_to_3(_cpu *);
extern void c_rg_fsubr_to_4(_cpu *);
extern void c_rg_fsubr_to_5(_cpu *);
extern void c_rg_fsubr_to_6(_cpu *);
extern void c_rg_fsubr_to_7(_cpu *);
extern void c_rg_fdivr_to_1(_cpu *);
extern void c_rg_fdivr_to_2(_cpu *);
extern void c_rg_fdivr_to_3(_cpu *);
extern void c_rg_fdivr_to_4(_cpu *);
extern void c_rg_fdivr_to_5(_cpu *);
extern void c_rg_fdivr_to_6(_cpu *);
extern void c_rg_fdivr_to_7(_cpu *);

static void check_registers(const _cpu *cpu, uint32_t flags, uint32_t stack) {
    assert(cpu->_ecx == 0x23456789 && cpu->_edx == 0x3456789a);
    assert(cpu->_ebp == 0x56789abc && cpu->_esi == 0x6789abcd);
    assert(cpu->_eflags == flags && cpu->_esp == stack);
}

int main(void) {
    Dl_info image;
    assert(dladdr((void *)(uintptr_t)c_rg_test, &image));
    uintptr_t base = (uintptr_t)image.dli_fbase;
    /* Code, guest heap and return sentinels share one 32-bit offset window.
       A general malloc arena can lie above the code and cannot be its base. */
    unsigned char *arena = mmap((void *)(base + 0x20000000), 65536,
        PROT_READ | PROT_WRITE, MAP_PRIVATE | MAP_ANONYMOUS, -1, 0);
    assert(arena != MAP_FAILED && (uintptr_t)arena > base);
    assert((uintptr_t)arena - base <= UINT32_MAX - 65536);
    _cpu *state = (_cpu *)(arena + 4096);
    uint32_t stack = (uint32_t)((uintptr_t)arena - base) + 65520;
    state->_pointer_offset = base;
    state->_esp = stack;
    state->_ebx = (uint32_t)((uintptr_t)arena - base) + 64;
    state->_ecx = 0x23456789; state->_edx = 0x3456789a;
    state->_ebp = 0x56789abc; state->_esi = 0x6789abcd;
    state->stack_bottom = arena; state->stack_top = arena + 65536;
    /* BSWAP must preserve both set and clear condition flags. */
    for (uint32_t bits = 0; bits < 64; ++bits) {
        uint32_t flags = 0x202 | ((bits & 1) ? CF : 0) | ((bits & 2) ? PF : 0) |
            ((bits & 4) ? AF : 0) | ((bits & 8) ? ZF : 0) |
            ((bits & 16) ? SF : 0) | ((bits & 32) ? OF : 0);
        state->_eflags = flags; state->_eax = 0x12345678; state->_edi = 0x89abcdef;
        c_rg_test(state);
        assert(state->_eax == 0x78563412 && state->_edi == 0x89abcdef);
        check_registers(state, flags, stack);
        c_rg_test_edi(state);
        assert(state->_edi == 0xefcdab89 && state->_eax == 0x78563412);
        check_registers(state, flags, stack);
    }
    uint32_t value = 0x12345678;
    memcpy(arena + 64, &value, sizeof value);
    c_rg_test_memory(state);
    memcpy(&value, arena + 64, sizeof value);
    assert(value == 0x78563412 && state->_eax == value);
    check_registers(state, state->_eflags, stack);
    float input = 1.25f, output;
    state->_st_cw = 0x37f;
    memcpy(arena + 64, &input, sizeof input);
    c_rg_test_float(state);
    memcpy(&output, arena + 64, sizeof output);
    assert(output == 2.5f && state->_st_top == 0 && state->_esp == stack);
    for (unsigned bit = 0; bit < 33; ++bit) {
        state->_eax = bit == 32 ? 0 : (1u << bit);
        uint32_t expected = state->_eax ? 0x22222222 : 0x33333333;
        c_rg_flags(state);
        assert(state->_eax == expected && state->_esp == stack);
    }
    const int32_t signed_cases[][2] = {
        {13,5}, {-13,5}, {13,-5}, {-13,-5}, {INT32_MIN,1},
        {INT32_MAX,-1}, {0,7}, {1,INT32_MIN}, {INT32_MIN,INT32_MAX}
    };
    const int32_t compare_cases[][2] = {
        {0,0}, {13,5}, {-13,5}, {13,-5}, {-13,-5},
        {INT32_MIN,INT32_MAX}, {INT32_MAX,INT32_MIN}, {INT32_MIN,INT32_MIN},
        {0,INT32_MIN}, {0,INT32_MAX}
    };
    for (unsigned i = 0; i < sizeof compare_cases / sizeof compare_cases[0]; ++i) {
        int32_t a = compare_cases[i][0], b = compare_cases[i][1];
        state->_eax = (uint32_t)a; state->_edi = (uint32_t)b;
        c_rg_compare(state);
        assert((int32_t)state->_eax == ((a > b) - (a < b)) && state->_esp == stack);
    }
    for (unsigned i = 0; i < sizeof signed_cases / sizeof signed_cases[0]; ++i) {
        int32_t a = signed_cases[i][0], b = signed_cases[i][1];
        state->_eax = (uint32_t)a; state->_edi = (uint32_t)b;
        c_rg_signed_divide(state);
        assert((int32_t)state->_eax == a / b && (int32_t)state->_edx == a % b);
        assert(state->_esp == stack);
    }
    const uint32_t unsigned_cases[][2] = {
        {13,5}, {UINT32_MAX,3}, {UINT32_MAX,UINT32_MAX}, {0,7},
        {0x80000000,1}, {1,0x80000000}
    };
    for (unsigned i = 0; i < sizeof unsigned_cases / sizeof unsigned_cases[0]; ++i) {
        uint32_t a = unsigned_cases[i][0], b = unsigned_cases[i][1];
        state->_eax = a; state->_edi = b;
        c_rg_unsigned_divide(state);
        assert(state->_eax == a / b && state->_edx == a % b && state->_esp == stack);
    }
    void (*reversed[2][7])(_cpu *) = {
        {c_rg_fsubr_to_1, c_rg_fsubr_to_2, c_rg_fsubr_to_3, c_rg_fsubr_to_4, c_rg_fsubr_to_5, c_rg_fsubr_to_6, c_rg_fsubr_to_7},
        {c_rg_fdivr_to_1, c_rg_fdivr_to_2, c_rg_fdivr_to_3, c_rg_fdivr_to_4, c_rg_fdivr_to_5, c_rg_fdivr_to_6, c_rg_fdivr_to_7}
    };
    /* Distinct operands detect a reversed subtraction or division. Every stack
       rotation also checks that the instruction neither pops nor touches ST0. */
    for (unsigned op = 0; op < 2; ++op) {
        for (unsigned reg = 1; reg < 8; ++reg) {
            for (unsigned top = 0; top < 8; ++top) {
                for (unsigned negative = 0; negative < 2; ++negative) {
                    double before[8];
                    for (unsigned i = 0; i < 8; ++i) before[i] = state->_st[i] = i + 2.0;
                    double first = negative ? -24.0 : 24.0;
                    before[top] = state->_st[top] = first;
                    unsigned dest = (top + reg) & 7;
                    double expected = op ? first / before[dest] : first - before[dest];
                    state->_st_top = top;
                    uint32_t flags = state->_eflags;
                    reversed[op][reg - 1](state);
                    assert(state->_st_top == top && state->_esp == stack && state->_eflags == flags);
                    for (unsigned i = 0; i < 8; ++i)
                        assert(state->_st[i] == (i == dest ? expected : before[i]));
                }
            }
        }
    }
    const double rounding[][5] = {
        {0.0, 0.0, 0.0, 0.0, 0.0}, {-0.0, -0.0, -0.0, -0.0, -0.0},
        {0.25, 0, 0, 1, 0}, {-0.25, -0.0, -1, -0.0, -0.0},
        {0.5, 0, 0, 1, 0}, {-0.5, -0.0, -1, -0.0, -0.0},
        {1.5, 2, 1, 2, 1}, {-1.5, -2, -2, -1, -1},
        {2.5, 2, 2, 3, 2}, {-2.5, -2, -3, -2, -2},
        {INFINITY, INFINITY, INFINITY, INFINITY, INFINITY},
        {-INFINITY, -INFINITY, -INFINITY, -INFINITY, -INFINITY},
        {NAN, NAN, NAN, NAN, NAN},
        {0x1p52, 0x1p52, 0x1p52, 0x1p52, 0x1p52},
        {-0x1p52, -0x1p52, -0x1p52, -0x1p52, -0x1p52}
    };
    int host_rounding = fegetround();
    assert(!fesetround(FE_UPWARD));
    for (unsigned mode = 0; mode < 4; ++mode) {
        for (unsigned i = 0; i < sizeof rounding / sizeof rounding[0]; ++i) {
            for (unsigned top = 0; top < 8; ++top) {
                state->_st_top = top;
                for (unsigned j = 0; j < 8; ++j) state->_st[j] = j + 100.0;
                state->_st[top] = rounding[i][0];
                uint32_t cw = state->_st_cw = 0x37f | (mode << 10);
                uint32_t flags = state->_eflags;
                c_rg_round(state);
                double got = state->_st[top], expected = rounding[i][mode + 1];
                assert(isnan(expected) ? isnan(got) : got == expected && !!signbit(got) == !!signbit(expected));
                assert(state->_st_top == top && state->_st_cw == cw && state->_esp == stack);
                assert(state->_eflags == flags && fegetround() == FE_UPWARD);
                for (unsigned j = 0; j < 8; ++j) if (j != top) assert(state->_st[j] == j + 100.0);
            }
        }
    }
    assert(!fesetround(host_rounding));
    /* The upstream stack stores doubles. Check the 80-bit wire format at
       that precision, including values that overflow/underflow a double. */
    const struct { uint64_t significand; uint16_t exponent; double expected; } extended[] = {
        {UINT64_C(0x8000000000000000), 0x3fff, 1.0},
        {UINT64_C(0xa000000000000000), 0xc000, -2.5},
        {UINT64_C(0xc90fdaa22168c235), 0x4000, 0x1.921fb54442d18p1},
        {0, 0, 0.0}, {0, 0x8000, -0.0},
        {UINT64_C(0x8000000000000000), 0x7fff, INFINITY},
        {UINT64_C(0x8000000000000000), 0xffff, -INFINITY},
        {UINT64_C(0xc000000000000000), 0x7fff, NAN},
        {1, 0, 0.0}, {UINT64_C(0x8000000000000000), 0x7ffe, INFINITY},
        {1, 0x4000, NAN}
    };
    for (unsigned i = 0; i < sizeof extended / sizeof extended[0]; ++i) {
        for (unsigned top = 0; top < 8; ++top) {
            for (unsigned j = 0; j < 8; ++j) state->_st[j] = j + 100.0;
            state->_st_top = top;
            for (unsigned j = 0; j < 8; ++j) arena[64 + j] = (unsigned char)(extended[i].significand >> (8 * j));
            arena[72] = extended[i].exponent & 255; arena[73] = extended[i].exponent >> 8;
            state->_ebx = (uint32_t)((uintptr_t)arena - base) + 64;
            uint32_t flags = state->_eflags;
            c_rg_load_extended(state);
            unsigned dest = (top - 1) & 7;
            double got = state->_st[dest], expected = extended[i].expected;
            assert(isnan(expected) ? isnan(got) : got == expected && !!signbit(got) == !!signbit(expected));
            assert(state->_st_top == dest && state->_esp == stack && state->_eflags == flags);
            for (unsigned j = 0; j < 8; ++j) if (j != dest) assert(state->_st[j] == j + 100.0);
        }
    }
    const double dividends[] = {-1000000, -63, -17, -15, -13, -1, -0.0, 0.0, 1, 13, 15, 17, 63, 1000000};
    for (unsigned i = 0; i < sizeof dividends / sizeof dividends[0]; ++i) {
        for (int divisor = -7; divisor <= 7; ++divisor) {
            if (!divisor) continue;
            for (unsigned top = 0; top < 8; ++top) {
                double value = dividends[i];
                for (unsigned j = 0; j < 8; ++j) state->_st[j] = j + 100.0;
                state->_st_top = top;
                state->_st[top] = value; state->_st[(top + 1) & 7] = divisor;
                uint32_t flags = state->_eflags;
                c_rg_remainder(state);
                double got = state->_st[top], expected = fmod(value, divisor);
                unsigned quotient = (unsigned)fabs(trunc(value / divisor)) & 7;
                unsigned condition = ((quotient & 4) ? 0x100 : 0) |
                    ((quotient & 2) ? 0x4000 : 0) | ((quotient & 1) ? 0x200 : 0);
                assert(got == expected && !!signbit(got) == !!signbit(expected));
                assert(state->_st_sw_cond == condition && state->_st_top == top);
                assert(state->_esp == stack && state->_eflags == flags);
                assert(state->_st[(top + 1) & 7] == divisor);
                for (unsigned j = 0; j < 8; ++j)
                    if (j != top && j != ((top + 1) & 7)) assert(state->_st[j] == j + 100.0);
            }
        }
    }
    const double angles[][2] = {
        {1, 1}, {-1, 1}, {1, -1}, {-1, -1}, {0.0, -1}, {-0.0, -1},
        {3, 2}, {-3, 2}, {INFINITY, 1}, {1, INFINITY}, {-INFINITY, -INFINITY}, {NAN, 1}
    };
    for (unsigned i = 0; i < sizeof angles / sizeof angles[0]; ++i) {
        for (unsigned top = 0; top < 8; ++top) {
            for (unsigned j = 0; j < 8; ++j) state->_st[j] = j + 100.0;
            state->_st_top = top;
            unsigned dest = (top + 1) & 7;
            state->_st[top] = angles[i][1]; state->_st[dest] = angles[i][0];
            uint32_t flags = state->_eflags;
            c_rg_atan(state);
            double got = state->_st[dest], expected = atan2(angles[i][0], angles[i][1]);
            assert(isnan(expected) ? isnan(got) : got == expected && !!signbit(got) == !!signbit(expected));
            assert(state->_st_top == dest && state->_esp == stack && state->_eflags == flags);
            for (unsigned j = 0; j < 8; ++j)
                if (j != top && j != dest) assert(state->_st[j] == j + 100.0);
        }
    }
    const double comparisons[] = {-INFINITY, -2.5, -0.0, 0.0, 2.5, INFINITY, NAN};
    for (unsigned i = 0; i < sizeof comparisons / sizeof comparisons[0]; ++i) {
        for (unsigned top = 0; top < 8; ++top) {
            for (unsigned j = 0; j < 8; ++j) state->_st[j] = j + 100.0;
            double value = comparisons[i];
            state->_st_top = top; state->_st[top] = value; state->_st_sw_cond = 0x4700;
            uint32_t flags = state->_eflags;
            c_rg_ftst(state);
            unsigned condition = isnan(value) ? 0x4500 : value == 0 ? 0x4000 : value < 0 ? 0x100 : 0;
            assert(state->_st_sw_cond == condition && state->_st_top == top);
            assert(state->_esp == stack && state->_eflags == flags);
            assert(isnan(value) ? isnan(state->_st[top]) : state->_st[top] == value);
            for (unsigned j = 0; j < 8; ++j) if (j != top) assert(state->_st[j] == j + 100.0);
        }
    }
    for (unsigned bits = 0; bits < 64; ++bits) {
        uint32_t flags = 0x202 | ((bits & 1) ? CF : 0) | ((bits & 2) ? PF : 0) |
            ((bits & 4) ? AF : 0) | ((bits & 8) ? ZF : 0) |
            ((bits & 16) ? SF : 0) | ((bits & 32) ? OF : 0);
        state->_eflags = flags;
        c_rg_jo(state);
        assert(state->_eax == ((flags & OF) ? 2u : 1u) && state->_eflags == flags && state->_esp == stack);
        c_rg_jno(state);
        assert(state->_eax == ((flags & OF) ? 1u : 2u) && state->_eflags == flags && state->_esp == stack);
        for (unsigned bit = 0; bit < 33; ++bit) {
            uint32_t count = bit == 32 ? 0 : 1u << bit;
            state->_ecx = count;
            c_rg_jecxz(state);
            assert(state->_eax == (count ? 1u : 2u) && state->_ecx == count);
            assert(state->_eflags == flags && state->_esp == stack);
        }
    }
    const struct { double value; uint64_t significand; uint16_t exponent; } stores[] = {
        {1.0, UINT64_C(0x8000000000000000), 0x3fff},
        {-2.5, UINT64_C(0xa000000000000000), 0xc000},
        {0x1.921fb54442d18p1, UINT64_C(0xc90fdaa22168c000), 0x4000},
        {0.0, 0, 0}, {-0.0, 0, 0x8000},
        {INFINITY, UINT64_C(0x8000000000000000), 0x7fff},
        {-INFINITY, UINT64_C(0x8000000000000000), 0xffff},
        {0x1p-1074, UINT64_C(0x8000000000000000), 0x3bcd},
        {0x1.fffffffffffffp1023, UINT64_C(0xfffffffffffff800), 0x43fe}
    };
    for (unsigned i = 0; i < sizeof stores / sizeof stores[0]; ++i) {
        for (unsigned top = 0; top < 8; ++top) {
            for (unsigned j = 0; j < 8; ++j) state->_st[j] = j + 100.0;
            state->_st_top = top; state->_st[top] = stores[i].value;
            state->_ebx = (uint32_t)((uintptr_t)arena - base) + 64;
            uint32_t flags = state->_eflags;
            memset(arena + 60, 0x5a, 20);
            c_rg_store_extended(state);
            for (unsigned j = 0; j < 8; ++j)
                assert(arena[64 + j] == (unsigned char)(stores[i].significand >> (8 * j)));
            assert(arena[72] == (stores[i].exponent & 255) && arena[73] == stores[i].exponent >> 8);
            for (unsigned j = 60; j < 64; ++j) assert(arena[j] == 0x5a);
            for (unsigned j = 74; j < 80; ++j) assert(arena[j] == 0x5a);
            assert(state->_st_top == ((top + 1) & 7) && state->_esp == stack && state->_eflags == flags);
            for (unsigned j = 0; j < 8; ++j) if (j != top) assert(state->_st[j] == j + 100.0);
            c_rg_load_extended(state);
            double got = state->_st[top];
            assert(state->_st_top == top && got == stores[i].value && !!signbit(got) == !!signbit(stores[i].value));
        }
    }
    assert(!munmap(arena, 65536));
    puts("PASS translated BSWAP EAX/EDI, all condition flags, offset memory, x87, shared-flag branches, inline signed/unsigned division reversed x87 destinations, all rounding modes, extended loads, remainder condition bits, atan quadrants, FTST, counter/overflow branches, extended stores and balanced guest returns");
    return 0;
}
