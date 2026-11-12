#include <assert.h>
#include <stdio.h>
#include <string.h>
#include <windows.h>
#include "guest_abi.h"
extern uint32_t rg_test_font10_metric(uint32_t source,uint32_t *changed_flags);
int main(void) {
    unsigned char *font=rg_low_alloc(256); assert(font);
    uint32_t changed;
    const uint16_t values[]={0,125,255,0xbeef,0xffff};
    for (unsigned i=0;i<sizeof values/sizeof values[0];++i) {
        memcpy(font+0x46,&values[i],2);
        assert(rg_test_font10_metric((uint32_t)(uintptr_t)font,&changed)==(0x12340000u|values[i]));
        assert(!changed);
    }
    assert(rg_test_font10_metric(0,&changed)==0x1234f000u);
    assert(!changed);
    assert(rg_low_free(font));
    puts("PASS style10 metric retains normal word/upper-register/flags behavior and measured DOS null value");
}
