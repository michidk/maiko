//! Historical execution-boundary diagnostic, separate from the game host.
// Compile the same reporter without linking the game runtime and its SDL/heap imports.
#[path = "../rust/crash.rs"]
mod crash;
use std::ffi::c_int;
unsafe extern "C" {
    fn rg_run();
}
#[unsafe(no_mangle)]
pub static mut X86_InterruptFlag: u32 = 0;
#[unsafe(no_mangle)]
pub extern "C" fn probe_interrupt(number: u32, eax: u32, return_pc: u32) -> ! {
    eprintln!(
        "BLOCKED original program requested INT {number:02x} EAX={eax:08x} translated_return={return_pc:08x}"
    );
    eprintln!(
        "DOS runtime is not implemented. This is an execution-boundary probe, not a playable port."
    );
    std::process::exit(2)
}
#[unsafe(no_mangle)]
pub extern "C" fn probe_boundary() -> ! {
    eprintln!("BLOCKED unimplemented SR runtime helper; menu not reached");
    std::process::exit(2)
}
/// # Safety
/// The linker supplies the diagnostic translated entry and its assembly stack.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rg_probe_main() -> c_int {
    crash::rg_crash_install();
    println!("Redguard SR diagnostic probe: entering statically translated RGFX entry point");
    unsafe {
        rg_run();
    }
    eprintln!("Unexpected return from DOS entry point");
    4
}
