#include <assert.h>
#include <stdint.h>
#include <stdio.h>
extern uint32_t rg_exe_variant, rg_skip_intro, rg_skip_outro; /* defined by the Rust runtime (install.rs) */
extern unsigned test_variant_flags(unsigned nonzero), test_variant_condition(unsigned nonzero), test_skip_call(unsigned nonzero);
int main(void) {
    for (unsigned set=0;set<=1;++set) {
        rg_exe_variant=rg_skip_intro=rg_skip_outro=set;
        for (unsigned nonzero=0;nonzero<=1;++nonzero) {
            assert(test_variant_flags(nonzero)==0x100+set);
            assert(test_variant_condition(nonzero)==(set || !nonzero));
            assert(test_skip_call(nonzero)==!set);
        }
    }
    rg_exe_variant=1;rg_skip_outro=0;
    assert(test_variant_flags(1)==0x101 && test_variant_condition(1)==0);
    puts("PASS variant and skip jumps follow their own flag, skipped and taken calls, original condition, flags and balanced stack");
}
