//! C ABI adapters for SR's CPU-state backend. Guest calls enter through an LLasm
//! global wrapper, which installs SR's return sentinel before the fastcc call.
use super::{GuestFrame, GuestRegs, guest_base, rg_host_service};
use maiko_runtime::llasm::Cpu;
use maiko_runtime::memory::{rg_low_alloc, rg_low_free};
use maiko_runtime::rg_log;

unsafe extern "C" {
    fn c_rg_dispatch(cpu: *mut Cpu);
}

/// # Safety
/// The platform CRT provides argc/argv on the initial thread.
#[unsafe(no_mangle)]
#[cfg(feature = "llasm-entry")]
pub unsafe extern "C" fn main(argc: i32, argv: *const *const std::ffi::c_char) -> i32 {
    unsafe { super::rg_native_main(argc, argv) }
}

/// # Safety
/// The target is a translated fastcc procedure in the guest address window and
/// `registers` describes a valid, writable guest stack with room for a return.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rg_call(target: u32, registers: *mut GuestRegs) {
    let registers = unsafe { &mut *registers };
    let base = guest_base();
    // Floating helpers return guest pointers into this state, so a host-stack
    // CPU would truncate those pointers outside the offset window.
    let state = rg_low_alloc(std::mem::size_of::<Cpu>()).cast::<Cpu>();
    if state.is_null() {
        rg_log!("BLOCKED could not allocate guest-addressable CPU state");
        std::process::abort();
    }
    let cpu = Cpu {
        registers: [
            registers.eax,
            registers.ecx,
            registers.edx,
            registers.ebx,
            registers.esp,
            registers.ebp,
            registers.esi,
            registers.edi,
            registers.flags,
        ],
        control: 0x37f,
        base: base as u64,
        result: i64::from(target),
        stack_bottom: base,
        stack_top: base + 0x8000_0000,
        ..Default::default()
    };
    unsafe {
        state.write(cpu);
        c_rg_dispatch(state);
    }
    let cpu = unsafe { &*state };
    *registers = GuestRegs {
        eax: cpu.registers[0],
        ecx: cpu.registers[1],
        edx: cpu.registers[2],
        ebx: cpu.registers[3],
        esp: cpu.registers[4],
        ebp: cpu.registers[5],
        esi: cpu.registers[6],
        edi: cpu.registers[7],
        flags: cpu.registers[8],
    };
    unsafe { rg_low_free(state.cast()) };
}

/// # Safety
/// Called once by the entry wrapper before translated code uses the x87 result.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rg_llasm_target(cpu: *const Cpu) -> u32 {
    unsafe { (*cpu).result as u32 }
}

/// # Safety
/// Called by a translated host boundary on the game thread with a live CPU and
/// readable guest arguments. Reentrant keyboard callbacks use their own CPU.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rg_llasm_service(cpu: *mut Cpu, service: u32) {
    let cpu = unsafe { &mut *cpu };
    let r = cpu.registers;
    let mut frame = GuestFrame {
        eax: u64::from(r[0]),
        ecx: u64::from(r[1]),
        edx: u64::from(r[2]),
        ebx: u64::from(r[3]),
        esp: u64::from(r[4]),
        ebp: u64::from(r[5]),
        esi: u64::from(r[6]),
        edi: u64::from(r[7]),
        flags: u64::from(r[8]),
    };
    unsafe { rg_host_service(service, &mut frame) };
    cpu.registers = [
        frame.eax as u32,
        frame.ecx as u32,
        frame.edx as u32,
        frame.ebx as u32,
        frame.esp as u32,
        frame.ebp as u32,
        frame.esi as u32,
        frame.edi as u32,
        frame.flags as u32,
    ];
}

/// # Safety
/// Same contract as `rg_llasm_service`; the interrupt number comes from SR.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn x86_int(cpu: *mut Cpu, interrupt: u32) {
    unsafe { rg_llasm_service(cpu, 0x1000 | interrupt) };
}

#[unsafe(no_mangle)]
pub extern "C" fn rg_llasm_variant() -> u32 {
    // Set during install selection before entry into translated code.
    unsafe { maiko_runtime::install::rg_exe_variant }
}

#[unsafe(no_mangle)]
pub extern "C" fn rg_translation_trap(address: u32) -> ! {
    rg_log!("BLOCKED unsupported original instruction at {address:08x}");
    std::process::abort()
}

#[unsafe(no_mangle)]
pub extern "C" fn X86_InPortProcedure(port: u16, size: u32) -> u32 {
    // Joystick port 0x201: no stick attached, every bit stays set so the poll times out.
    if port == 0x201 {
        return u32::MAX;
    }
    rg_log!("BLOCKED untranslated hardware input port={port:04x} size={size}");
    std::process::abort()
}

#[unsafe(no_mangle)]
pub extern "C" fn X86_OutPortProcedure(port: u16, size: u32, value: u32) {
    if port == 0x201 {
        return;
    }
    rg_log!("BLOCKED untranslated hardware output port={port:04x} size={size} value={value:08x}");
    std::process::abort()
}

#[unsafe(no_mangle)]
pub extern "C" fn X86_ReadFsDword(address: u32) -> ! {
    rg_log!("BLOCKED untranslated FS read address={address:08x}");
    std::process::abort()
}

#[unsafe(no_mangle)]
pub extern "C" fn X86_WriteFsDword(address: u32, value: u32) -> ! {
    rg_log!("BLOCKED untranslated FS write address={address:08x} value={value:08x}");
    std::process::abort()
}

#[unsafe(no_mangle)]
pub extern "C" fn X86_ReadMemProcedure(address: u32, size: u32) -> ! {
    rg_log!("BLOCKED untranslated DOS memory read address={address:08x} size={size}");
    std::process::abort()
}

#[unsafe(no_mangle)]
pub extern "C" fn X86_WriteMemProcedure(address: u32, size: u32, value: u32) -> ! {
    rg_log!(
        "BLOCKED untranslated DOS memory write address={address:08x} size={size} value={value:08x}"
    );
    std::process::abort()
}
