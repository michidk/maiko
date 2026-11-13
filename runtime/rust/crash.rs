//! Fatal reporting without allocation, locks, unwinding or exit cleanup.
use std::fmt::{self, Write};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering::Relaxed};
#[unsafe(no_mangle)]
pub static rg_call_head: AtomicU32 = AtomicU32::new(0);
#[unsafe(no_mangle)]
pub static rg_call_ring: [AtomicU32; 64] = [const { AtomicU32::new(0) }; 64];
static REPORTING: AtomicBool = AtomicBool::new(false);
struct Report {
    bytes: [u8; 4096],
    length: usize,
}
impl Report {
    fn new() -> Self {
        Self {
            bytes: [0; 4096],
            length: 0,
        }
    }
    fn calls(&mut self) {
        let head = rg_call_head.load(Relaxed);
        let count = head.min(32);
        let mut saved = [0; 32];
        for (i, value) in saved.iter_mut().take(count as usize).enumerate() {
            *value = rg_call_ring[(head.wrapping_sub(count).wrapping_add(i as u32) & 63) as usize]
                .load(Relaxed);
        }
        for value in saved.into_iter().take(count as usize) {
            let _ = writeln!(self, "CALLSITE {value:016x}");
        }
    }
    fn output(&self) {
        #[cfg(unix)]
        unsafe {
            libc::write(2, self.bytes.as_ptr().cast(), self.length);
        }
        #[cfg(windows)]
        unsafe {
            use windows_sys::Win32::{Storage::FileSystem::WriteFile, System::Console::*};
            let mut written = 0;
            WriteFile(
                GetStdHandle(STD_ERROR_HANDLE),
                self.bytes.as_ptr(),
                self.length as u32,
                &mut written,
                std::ptr::null_mut(),
            );
        }
    }
}
impl Write for Report {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        if s.len() > self.bytes.len() - self.length {
            return Err(fmt::Error);
        }
        self.bytes[self.length..self.length + s.len()].copy_from_slice(s.as_bytes());
        self.length += s.len();
        Ok(())
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn rg_dump_calls() {
    let mut report = Report::new();
    report.calls();
    report.output();
}
#[cfg(windows)]
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
#[cfg(unix)]
static mut SIGNAL_STACK: [u8; 65536] = [0; 65536];
#[cfg(unix)]
extern "C" fn fault(signal: i32, info: *mut libc::siginfo_t, context: *mut std::ffi::c_void) {
    if REPORTING.swap(true, Relaxed) {
        unsafe {
            libc::_exit(3);
        }
    }
    let context = unsafe { &*context.cast::<libc::ucontext_t>() };
    let regs = &context.uc_mcontext.gregs;
    let mut report = Report::new();
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
    report.calls();
    report.output();
    unsafe {
        libc::_exit(3);
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn rg_crash_install() {
    #[cfg(windows)]
    unsafe {
        use windows_sys::Win32::System::Diagnostics::Debug::*;
        SetErrorMode(SEM_FAILCRITICALERRORS | SEM_NOOPENFILEERRORBOX | SEM_NOGPFAULTERRORBOX);
        SetUnhandledExceptionFilter(Some(fault));
    }
    #[cfg(unix)]
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
