//! Native signal context formatting into the shared fixed-size report.
use crate::crash::Report;
use std::fmt::Write;
pub(crate) fn report_context(
    report: &mut Report,
    signal: i32,
    info: *mut libc::siginfo_t,
    context: &libc::ucontext_t,
) {
    #[cfg(target_arch = "x86_64")]
    {
        let regs = &context.uc_mcontext.gregs;
        let _ = writeln!(
            report,
            "BLOCKED Linux signal={signal:016x} host_pc={:016x} guest_sp={:016x} address={:016x}",
            regs[libc::REG_RIP as usize],
            regs[libc::REG_R11 as usize],
            unsafe { (*info).si_addr() } as usize
        );
        for (name, index) in [
            ("eax", libc::REG_RAX),
            ("ebx", libc::REG_RBX),
            ("ecx", libc::REG_RCX),
            ("edx", libc::REG_RDX),
            ("esi", libc::REG_RSI),
            ("edi", libc::REG_RDI),
            ("ebp", libc::REG_RBP),
        ] {
            let _ = write!(report, " {name}={:016x}", regs[index as usize]);
        }
        let _ = writeln!(report);
    }
    #[cfg(not(target_arch = "x86_64"))]
    {
        let _ = context;
        let _ = writeln!(
            report,
            "BLOCKED Linux signal={signal:016x} address={:016x}",
            unsafe { (*info).si_addr() } as usize
        );
    }
}
