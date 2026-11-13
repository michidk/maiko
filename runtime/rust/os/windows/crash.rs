//! Native fatal error handling. No allocation, locks or exit cleanup.
use crate::crash::{REPORTING, Report};
use std::fmt::Write;
use std::sync::atomic::Ordering::Relaxed;
unsafe extern "system" fn fault(
    info: *const windows_sys::Win32::System::Diagnostics::Debug::EXCEPTION_POINTERS,
) -> i32 {
    use windows_sys::Win32::{
        Foundation::*,
        System::{Diagnostics::Debug::*, Threading::*},
    };
    if REPORTING.swap(true, Relaxed) {
        unsafe {
            TerminateProcess(GetCurrentProcess(), 3);
        }
        return EXCEPTION_EXECUTE_HANDLER;
    }
    let info = unsafe { &*info };
    let exception = unsafe { &*info.ExceptionRecord };
    let context = unsafe { &*info.ContextRecord };
    let mut report = Report::new();
    let _ = writeln!(
        report,
        "BLOCKED exception={:08x} host_pc={:p} context_rip={:08x} guest_sp={:08x}",
        exception.ExceptionCode, exception.ExceptionAddress, context.Rip, context.R11
    );
    if (exception.ExceptionCode == EXCEPTION_ACCESS_VIOLATION
        || exception.ExceptionCode == EXCEPTION_IN_PAGE_ERROR)
        && exception.NumberParameters >= 2
    {
        let _ = writeln!(
            report,
            "Memory operation={} address={:x}",
            exception.ExceptionInformation[0], exception.ExceptionInformation[1]
        );
    }
    let _ = writeln!(
        report,
        "REGS eax={:08x} ebx={:08x} ecx={:08x} edx={:08x} esi={:08x} edi={:08x} ebp={:08x} r8={:08x} r9={:08x}",
        context.Rax,
        context.Rbx,
        context.Rcx,
        context.Rdx,
        context.Rsi,
        context.Rdi,
        context.Rbp,
        context.R8,
        context.R9
    );
    let fpu = unsafe { &context.Anonymous.FltSave };
    let _ = writeln!(
        report,
        "FPU thread={} control={:04x} status={:04x} tag={:02x} error={:08x} data={:08x} mxcsr={:08x}",
        unsafe { GetCurrentThreadId() },
        fpu.ControlWord,
        fpu.StatusWord,
        fpu.TagWord,
        fpu.ErrorOffset,
        fpu.DataOffset,
        context.MxCsr
    );
    report.calls();
    report.output();
    unsafe {
        TerminateProcess(GetCurrentProcess(), 3);
    }
    EXCEPTION_EXECUTE_HANDLER
}
pub(crate) fn install_crash() {
    unsafe {
        use windows_sys::Win32::System::Diagnostics::Debug::*;
        SetErrorMode(SEM_FAILCRITICALERRORS | SEM_NOOPENFILEERRORBOX | SEM_NOGPFAULTERRORBOX);
        SetUnhandledExceptionFilter(Some(fault));
    }
}
pub(crate) fn write_stderr(bytes: &[u8]) {
    unsafe {
        use windows_sys::Win32::{Storage::FileSystem::WriteFile, System::Console::*};
        let mut written = 0;
        WriteFile(
            GetStdHandle(STD_ERROR_HANDLE),
            bytes.as_ptr(),
            bytes.len() as u32,
            &mut written,
            std::ptr::null_mut(),
        );
    }
}
