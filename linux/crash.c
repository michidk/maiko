#define _GNU_SOURCE
#include "crash.h"
#include <stdint.h>
#include <stdio.h>
#include <signal.h>
#include <unistd.h>
#include <ucontext.h>
volatile uint32_t rg_call_head,rg_call_ring[64];
static volatile sig_atomic_t reporting;
static unsigned char signal_stack[65536];
void rg_dump_calls(void) {
    uint32_t head=rg_call_head,count=head<32 ? head : 32,saved[32];
    for(uint32_t i=0;i<count;++i) saved[i]=rg_call_ring[(head-count+i)&63];
    for(uint32_t i=0;i<count;++i) fprintf(stderr,"CALLSITE %08x\n",saved[i]);
}
static char *literal(char *p,const char *text) {while(*text) *p++=*text++;return p;}
static char *hex(char *p,uint64_t value) {
    for(int shift=60;shift>=0;shift-=4) *p++="0123456789abcdef"[(value>>shift)&15];
    return p;
}
static void fault(int signal,siginfo_t *info,void *context) {
    if(reporting) _exit(3);
    reporting=1;
    ucontext_t *state=context;char buffer[4096],*p=buffer;
    p=literal(p,"BLOCKED Linux signal=");p=hex(p,(unsigned)signal);
    p=literal(p," host_pc=");p=hex(p,state->uc_mcontext.gregs[REG_RIP]);
    p=literal(p," guest_sp=");p=hex(p,state->uc_mcontext.gregs[REG_R11]);
    p=literal(p," address=");p=hex(p,(uintptr_t)info->si_addr);*p++='\n';
    const int indexes[]={REG_RAX,REG_RBX,REG_RCX,REG_RDX,REG_RSI,REG_RDI,REG_RBP};
    const char *names[]={" eax="," ebx="," ecx="," edx="," esi="," edi="," ebp="};
    for(unsigned i=0;i<7;++i) {p=literal(p,names[i]);p=hex(p,state->uc_mcontext.gregs[indexes[i]]);}*p++='\n';
    uint32_t head=rg_call_head,count=head<32 ? head : 32;
    for(uint32_t i=0;i<count;++i) {p=literal(p,"CALLSITE ");p=hex(p,rg_call_ring[(head-count+i)&63]);*p++='\n';}
    /* write and _exit are async-signal-safe, unlike stdio or heap allocation. */
    ssize_t ignored=write(STDERR_FILENO,buffer,(size_t)(p-buffer));(void)ignored;_exit(3);
}
void rg_crash_install(void) {
    stack_t stack={0};stack.ss_sp=signal_stack;stack.ss_size=sizeof signal_stack;
    if(sigaltstack(&stack,NULL)) _exit(3);
    struct sigaction action={0};action.sa_sigaction=fault;action.sa_flags=SA_SIGINFO|SA_ONSTACK;
    sigemptyset(&action.sa_mask);
    const int signals[]={SIGSEGV,SIGBUS,SIGILL,SIGFPE,SIGABRT};
    for(unsigned i=0;i<sizeof signals/sizeof signals[0];++i) if(sigaction(signals[i],&action,NULL)) _exit(3);
}
