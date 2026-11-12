#include <assert.h>
#include <stdint.h>
#include <stdio.h>
uint32_t rg_exe_variant;
extern unsigned test_variant_flags(unsigned nonzero), test_variant_condition(unsigned nonzero);
int main(void) {
    for (rg_exe_variant=0;rg_exe_variant<=1;++rg_exe_variant) {
        for (unsigned nonzero=0;nonzero<=1;++nonzero) {
            assert(test_variant_flags(nonzero)==0x100+rg_exe_variant);
            assert(test_variant_condition(nonzero)==(rg_exe_variant || !nonzero));
        }
    }
    puts("PASS both image branch variants, original condition, flags and balanced stack");
}
