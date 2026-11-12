#ifndef RG_GUEST_ABI_H
#define RG_GUEST_ABI_H
#include <stddef.h>
#include <stdint.h>
#ifdef __cplusplus
extern "C" {
#endif
typedef struct {
    uint32_t eax, ebx, ecx, edx, esi, edi, ebp, esp, flags;
} GuestRegs;
typedef struct {
    uint64_t esp, ebp, edi, esi, edx, ecx, ebx, eax, flags;
} GuestFrame;
int rg_guest_pointer(const void *pointer, size_t size, uint32_t *out);
void *rg_low_alloc(size_t size);
int rg_low_free(void *pointer);
size_t rg_low_size(const void *pointer);
void rg_call(uint32_t target, GuestRegs *regs);
void rg_service(uint32_t service, GuestFrame *frame);
#ifdef __cplusplus
}
#endif
#endif
