//! Native fatal error handling. No allocation, locks or exit cleanup.
use crate::crash::{REPORTING, Report};
use std::sync::atomic::Ordering::Relaxed;
static mut SIGNAL_STACK: [u8; 65536] = [0; 65536];
extern "C" fn fault(signal: i32, info: *mut libc::siginfo_t, context: *mut std::ffi::c_void) {
    if REPORTING.swap(true, Relaxed) {
        unsafe {
            libc::_exit(3);
        }
    }
    let context = unsafe { &*context.cast::<libc::ucontext_t>() };
    let mut report = Report::new();
    crate::os::report_context(&mut report, signal, info, context);
    report.calls();
    report.output();
    unsafe {
        libc::_exit(3);
    }
}
pub(crate) fn install_crash() {
    unsafe {
        let stack = libc::stack_t {
            ss_sp: std::ptr::addr_of_mut!(SIGNAL_STACK).cast(),
            ss_flags: 0,
            ss_size: 65536,
        };
        if libc::sigaltstack(&stack, std::ptr::null_mut()) != 0 {
            libc::_exit(3);
        }
        let mut action: libc::sigaction = std::mem::zeroed();
        action.sa_sigaction = fault as usize;
        action.sa_flags = libc::SA_SIGINFO | libc::SA_ONSTACK;
        libc::sigemptyset(&mut action.sa_mask);
        for signal in [
            libc::SIGSEGV,
            libc::SIGBUS,
            libc::SIGILL,
            libc::SIGFPE,
            libc::SIGABRT,
        ] {
            if libc::sigaction(signal, &action, std::ptr::null_mut()) != 0 {
                libc::_exit(3);
            }
        }
    }
}
pub(crate) fn write_stderr(bytes: &[u8]) {
    unsafe {
        libc::write(2, bytes.as_ptr().cast(), bytes.len());
    }
}
