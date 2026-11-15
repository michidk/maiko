//! Native signal context formatting into the shared fixed-size report.
use crate::crash::Report;
use std::fmt::Write;
pub(crate) fn report_context(
    report: &mut Report,
    signal: i32,
    info: *mut libc::siginfo_t,
    context: &libc::ucontext_t,
) {
    let _ = writeln!(
        report,
        "BLOCKED macOS signal={signal:016x} address={:016x}",
        unsafe { (*info).si_addr() } as usize
    );
    if !context.uc_mcontext.is_null() {
        let state = unsafe { &(*context.uc_mcontext).__ss };
        #[cfg(target_arch = "aarch64")]
        {
            let _ = writeln!(
                report,
                "HOST pc={:016x} sp={:016x} fp={:016x} lr={:016x}",
                state.__pc, state.__sp, state.__fp, state.__lr
            );
        }
        #[cfg(target_arch = "x86_64")]
        {
            let _ = writeln!(
                report,
                "HOST pc={:016x} sp={:016x} bp={:016x}",
                state.__rip, state.__rsp, state.__rbp
            );
        }
    }
}
