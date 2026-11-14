//! Original-game entry and host-service dispatch. All game logic stays translated.
use maiko_runtime::rg_log;
#[cfg(feature = "llasm")]
mod llasm;
use maiko_runtime::memory::{guest_address, guest_ptr};
use maiko_runtime::{
    audio::*,
    crash::*,
    files::rg_file_service,
    guest::{GuestFrame, GuestRegs},
    host::*,
    input::{overlay, *},
    install::{rg_install_select, rg_skip_intro, rg_skip_outro},
    memory::*,
    platform::*,
    storage::{rg_host_chdir, rg_host_cwd},
    timing::*,
};
use std::ffi::{CStr, c_char, c_int, c_void};
use std::io::{self, Write};
unsafe extern "C" {
    fn rg_call(target: u32, regs: *mut GuestRegs);
    fn rg_init();
    fn rg_main();
    fn rg_key_event();
    fn host_movie_clock_read();
    fn host_movie_clock_stop();
    static mut g_1A9EC8: u32;
    static mut g_1A9ECC: u32;
    static mut g_1A9EB8: u32;
    static mut g_1A9EBC: u32;
    static mut g_1A9EC0: u32;
    static mut g_1A9EE9: u32;
    static g_34131C: u32;
    static g_341320: u32;
    static debug_type: u32;
    static object_asm_size: u32;
    static object_c_size: u32;
    static final_version: u32;
    static memory_manager: u32;
    static mut keyboard_extended_handler: u8;
    static mut keyboard_caps: u8;
    static mut keyboard_scroll: u8;
    static mut keyboard_num: u8;
    static mut keyboard_initialized: u16;
    static mut keyboard_state: [u16; 128];
    static mut rg_movie_clock_callback: u32;
    static mut rg_movie_clock_shutdown: u32;
    static mut rg_sound_driver: u32;
    static mut rg_sound_open: u32;
    static mut rg_sound_handles: [u32; 16];
    static loc_1A3865: u8;
    static loc_1A824C: u8;
    static loc_1A4620: u8;
    static loc_1A4624: u8;
    static mut loc_1A4654: u8;
    static mut loc_1EEC50: u8;
    static mut loc_1EEC58: u8;
    static mut loc_1EEC5C: u8;
    static mut loc_1EEC60: u8;
    static mut loc_1EEC64: u8;
    static mut loc_1EEC68: u8;
    static loc_1EEC48: u8;
    static loc_1EEC54: u8;
    static loc_1F29C4: u8;
    static loc_1F29E0: u8;
    static loc_1A3F5C: u8;
    static rg_tick_delta: u32;
}
#[unsafe(no_mangle)]
pub static mut X86_InterruptFlag: u32 = 0;
// The translated game enters this dispatcher only on its owning game thread.
// Never borrow this state across rg_call: keyboard dispatch can re-enter it.
static mut KEYBOARD_INSTALLED: bool = false;
static mut KEY_STACK: u32 = 0;
static mut ALLOCATIONS: u32 = 0;
static TRACE_ALLOC: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
static mut SIGNAL_HANDLERS: [u32; 13] = [0; 13];
static mut KEYBOARD_LOCKS: u32 = 0;
static mut HARD_ERROR_CALLBACK: u32 = 0;
static mut CRITICAL_VECTOR: u32 = 0;
unsafe fn read32(p: *const u8) -> u32 {
    unsafe { p.cast::<u32>().read_unaligned() }
}
unsafe fn ctext(p: u32) -> String {
    unsafe { CStr::from_ptr(guest_ptr::<c_char>(p)) }
        .to_string_lossy()
        .into_owned()
}
fn close_requested() {
    if rg_input_pump() == 0 {
        rg_log!("event=exit-request reason=window-close");
        rg_clock_stop();
        maiko_runtime::host::exit_cleanly(0);
    }
}
#[derive(Default)]
struct Observation {
    frames: u32,
    rows: u32,
    last: u32,
    actor: u32,
    startup: u32,
    ui: u32,
}
thread_local! {static OBSERVATION:std::cell::RefCell<Observation>=std::cell::RefCell::new(Observation::default());}
unsafe fn runtime_state() {
    unsafe {
        OBSERVATION.with(|state| {
        let mut state=state.borrow_mut();
        let now=rg_clock_milliseconds();let startup=read32(&raw const loc_1A4624);let actor=read32(&raw const loc_1A3865);let ui=read32(&raw const loc_1A824C);state.frames=state.frames.wrapping_add(1);
        if state.rows>=2048 || (state.frames>16 && actor==state.actor && ui==state.ui && startup==state.startup && now.wrapping_sub(state.last)<250) {return;}
        let mut text=format!("STATE ms={now} frame={} ui={ui} actor={actor:08x} startup={startup} whole={} fine={} scale={} fps={} fly={} vertical_bypass={}",state.frames,rg_tick_delta,read32(&raw const loc_1A4620),read32(&raw const loc_1EEC48),read32(&raw const loc_1EEC54),read32(&raw const loc_1F29C4),read32(&raw const loc_1F29E0));
        let p=guest_ptr::<u8>(actor);
        if rg_host_readable(p.cast(),0x276)!=0 {
            let a=guest_ptr::<u8>(read32(p.add(0x247)));
            text+=&format!(" xyz={},{},{} motion={},{},{} origin={} timer={} jump_key={}",read32(p.add(0xcb)) as i32,read32(p.add(0xcf)) as i32,read32(p.add(0xd3)) as i32,*p.add(0x1b1),*p.add(0x1b2),*p.add(0x1b3),read32(p.add(0x1c2)) as i32,*p.add(0x1c6),keyboard_state[0x39]);
            if rg_host_readable(a.cast(),12)!=0 {text+=&format!(" anim={} model_vertical={}",a.cast::<u16>().read_unaligned(),*a.add(11));}
        }
        rg_log!("{text}");state.last=now;state.actor=actor;state.startup=startup;state.ui=ui;state.rows+=1;
    });
    }
}
fn overlay_panel(ui: &mut overlay::egui::Ui) {
    unsafe {
        ui.label(format!("World {}", read32(&raw const loc_1A3F5C) as i32));
        let actor = read32(&raw const loc_1A3865) as usize as *const u8;
        if rg_host_readable(actor.cast(), 0xd7) != 0 {
            ui.label(format!(
                "Cyrus at {}, {}, {}",
                read32(actor.add(0xcb)) as i32,
                read32(actor.add(0xcf)) as i32,
                read32(actor.add(0xd3)) as i32
            ));
        }
        ui.label(format!(
            "Skip intro {}, skip outro {}",
            rg_skip_intro != 0,
            rg_skip_outro != 0
        ));
    }
}
/// # Safety
/// Called on the game thread with a valid frame, mapped guest stack and all
/// translated data symbols initialized. Re-entry during key delivery is allowed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rg_host_service(service: u32, frame: *mut GuestFrame) {
    unsafe {
        let f = &mut *frame;
        if service & 0x80000000 != 0 {
            if service == 0x800d92d0 {
                static mut COUNT: u32 = 0;
                if COUNT >= 32 {
                    return;
                }
                let p = guest_ptr::<u8>(f.eax as u32);
                let readable = rg_host_readable(p.cast(), 12) != 0;
                if COUNT < 8 || (!readable && COUNT < 32) {
                    let count = COUNT;
                    let vector = if readable {
                        format!(
                            " xyz={},{},{}",
                            p.cast::<f32>().read_unaligned(),
                            p.add(4).cast::<f32>().read_unaligned(),
                            p.add(8).cast::<f32>().read_unaligned()
                        )
                    } else {
                        String::new()
                    };
                    rg_log!(
                        "CLIP entry={count} vector={:08x} ecx={:08x} edx={:08x} readable={}{vector}",
                        f.eax as u32,
                        f.ecx as u32,
                        f.edx as u32,
                        u8::from(readable)
                    );
                }
                if COUNT < 32 {
                    COUNT += 1;
                }
                return;
            }
            if service == 0x8002bd78 {
                if std::env::var_os("RG_TRACE_INI").is_some() {
                    rg_log!(
                        "INI query file={} section={} key={}",
                        ctext(f.eax as u32),
                        ctext(f.edx as u32),
                        ctext(f.ebx as u32)
                    );
                }
                return;
            }
            rg_log!(
                "TRACE original={:x} eax={:08x} edx={:08x}",
                service & 0x7fffffff,
                f.eax as u32,
                f.edx as u32
            );
            return;
        }
        if [23, 8, 0x200c].contains(&service) {
            close_requested();
        }
        if KEYBOARD_INSTALLED {
            loop {
                let scan = rg_input_take_scan();
                if scan < 0 {
                    break;
                }
                let mut key = GuestRegs {
                    eax: scan as u32,
                    esp: KEY_STACK,
                    ..Default::default()
                };
                rg_call(guest_address(rg_key_event as *const ()), &mut key);
            }
        }
        if service == 0x301b {
            if rg_movie_clock_callback == guest_address(host_movie_clock_read as *const ()) {
                rg_movie_clock_callback = 0;
            }
            return;
        }
        if service & 0xf000 == 0x2000 {
            if maiko_glide::rg_glide_service(service - 0x2000, f) != 0 {
                if service == 0x200c {
                    runtime_state();
                }
                return;
            }
            close_requested();
        }
        if service & 0xff00 == 0x3100
            && maiko_runtime::redbook::rg_redbook_service(service & 0xff, f) != 0
        {
            return;
        }
        if service & 0xff00 == 0x3000 {
            if service == 0x3000 {
                if rg_audio_open((&raw mut rg_sound_handles).cast()) != 0 {
                    let mut driver = GuestFrame::default();
                    rg_audio_service(19, &mut driver);
                    rg_sound_driver = driver.eax as u32;
                    rg_sound_open = 1;
                    return;
                }
            } else if service == 0x3001 {
                rg_audio_close();
                rg_sound_driver = 0;
                rg_sound_open = 0;
                rg_sound_handles = [0; 16];
                return;
            } else if rg_audio_service(service - 0x3000, f) != 0 {
                return;
            }
        }
        match service {
            27 => {
                let remaining = read32(&raw const loc_1A4624);
                pace_startup_clock(remaining);
                // Feed the game the frame rate of the last real frames instead of its lagging one-second
                // measurement, which loads, menus and the 12 Hz warm-up foul (see `FrameGuard`).
                let ticks =
                    maiko_runtime::timing::rg_bios_ticks.load(std::sync::atomic::Ordering::Relaxed);
                let now_ms = rg_clock_milliseconds();
                let fixed = maiko_runtime::timing::FRAME_GUARD
                    .with(|guard| guard.borrow_mut().step(remaining != 0, now_ms, ticks));
                static CALLS: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
                CALLS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                if let Some(t) = fixed {
                    static LAST_LOG: std::sync::atomic::AtomicU32 =
                        std::sync::atomic::AtomicU32::new(0);
                    let last = LAST_LOG.load(std::sync::atomic::Ordering::Relaxed);
                    if now_ms.wrapping_sub(last) >= 2000 {
                        LAST_LOG.store(now_ms, std::sync::atomic::Ordering::Relaxed);
                        maiko_runtime::rg_log!(
                            "event=frame-timing fps={} scale={} calls={} (game had measured {} smoothed {} scale {})",
                            t.measured,
                            t.scale,
                            CALLS.swap(0, std::sync::atomic::Ordering::Relaxed),
                            read32(&raw const loc_1EEC5C) as i32,
                            read32(&raw const loc_1EEC58) as i32,
                            read32(&raw const loc_1EEC50) as i32
                        );
                    }
                    let put = |p: *mut u8, v: u32| (p).cast::<u32>().write_unaligned(v);
                    put(&raw mut loc_1EEC58, t.smoothed as u32);
                    put(&raw mut loc_1EEC5C, t.measured as u32);
                    put(&raw mut loc_1EEC60, t.frames as u32);
                    put(&raw mut loc_1EEC64, t.baseline);
                    put(&raw mut loc_1EEC68, t.remainder as u32);
                    put(&raw mut loc_1EEC50, t.scale as u32);
                }
                // Original flag at loc_1A4654 turns the frame-time smoothing on; see SmoothingRelease.
                let flag = read32(&raw const loc_1A4654);
                let action = maiko_runtime::timing::SMOOTHING
                    .with(|release| release.borrow_mut().step(remaining, flag));
                match action {
                    maiko_runtime::timing::Smoothing::Keep => {}
                    maiko_runtime::timing::Smoothing::Disable => {
                        (&raw mut loc_1A4654).cast::<u32>().write_unaligned(0)
                    }
                    maiko_runtime::timing::Smoothing::Restore(value) => {
                        (&raw mut loc_1A4654).cast::<u32>().write_unaligned(value)
                    }
                }
                return;
            }
            24 => {
                rg_movie_clock_callback = guest_address(host_movie_clock_read as *const ());
                rg_movie_clock_shutdown = guest_address(host_movie_clock_stop as *const ());
                f.eax = 0;
                return;
            }
            25 => {
                f.eax = rg_clock_milliseconds().into();
                return;
            }
            26 | 23 => return,
            22 => {
                let capacity = rg_low_size(guest_ptr::<c_void>(f.eax as u32));
                if capacity != 0 {
                    f.eax = capacity as u64;
                    return;
                }
            }
            21 => {
                rg_log!(
                    "Legacy VGA DAC clear has no effect on the native Glide display; original Glide palette upload retained"
                );
                return;
            }
            20 => {
                if !rg_input_window(640, 480).is_null() {
                    rg_log!(
                        "Native window prepared for original Glide initialization; VGA mode13 unavailable"
                    );
                    return;
                }
            }
            19 => {
                CRITICAL_VECTOR = f.eax as u32;
                f.eax = 0;
                rg_log!(
                    "Native critical-file policy replaces DOS vector 24h; errors return status"
                );
                return;
            }
            16 => {
                if KEY_STACK == 0 {
                    let p = rg_low_alloc(65536);
                    if p.is_null() {
                        blocked(service, f);
                    }
                    KEY_STACK = guest_address(p) + 65536;
                }
                keyboard_extended_handler = 1;
                KEYBOARD_INSTALLED = true;
                rg_log!(
                    "Native keyboard event delivery installed; original key translation retained"
                );
                return;
            }
            17 => {
                keyboard_caps = 0;
                keyboard_scroll = 0;
                keyboard_num = 0;
                return;
            }
            18 => {
                KEYBOARD_INSTALLED = false;
                keyboard_initialized = 0;
                keyboard_state = [0; 128];
                return;
            }
            15 => {
                rg_platform_info(f);
                return;
            }
            13 => {
                HARD_ERROR_CALLBACK = f.eax as u32;
                rg_log!("Native file errors return status without a DOS critical-error dialog");
                return;
            }
            14 => {
                HARD_ERROR_CALLBACK = 0;
                return;
            }
            11 | 12 => {
                f.eax = rg_host_lock(
                    guest_ptr::<c_void>(f.eax as u32),
                    f.edx as u32 as usize,
                    i32::from(service == 12),
                ) as u64;
                return;
            }
            9 => {
                rg_log!(
                    "Original SYSTEM.INI parsed final_version={} memory_manager={}",
                    final_version,
                    memory_manager
                );
                KEYBOARD_LOCKS &= !0x10;
                rg_log!("Native input scroll lock cleared");
                return;
            }
            10 => {
                KEYBOARD_LOCKS &= !0x20;
                rg_log!("Native input num lock cleared");
                return;
            }
            8 => {
                if rg_int386(f) != 0 {
                    return;
                }
                close_requested();
            }
            1 => {
                let size = f.eax as u32;
                let p = rg_low_alloc(size as usize);
                let mut result = 0;
                if !p.is_null() && rg_guest_pointer(p, size as usize, &mut result) == 0 {
                    std::process::abort();
                }
                f.eax = guest_address(p).into();
                if *TRACE_ALLOC.get_or_init(|| std::env::var_os("RG_TRACE_ALLOC").is_some()) {
                    rg_log!(
                        "ALLOC size={size} pointer={:08x} caller={:08x}",
                        p as usize,
                        read32(guest_ptr::<u8>(f.esp as u32))
                    );
                }
                ALLOCATIONS = ALLOCATIONS.wrapping_add(1);
                if ALLOCATIONS < 12 {
                    rg_log!("malloc({size}) -> {:08x}", f.eax);
                }
                return;
            }
            2 => {
                let p = guest_ptr::<c_void>(f.eax as u32);
                if *TRACE_ALLOC.get_or_init(|| std::env::var_os("RG_TRACE_ALLOC").is_some()) {
                    rg_log!(
                        "FREE pointer={:08x} size={} caller={:08x}",
                        f.eax,
                        rg_low_size(p),
                        read32(guest_ptr::<u8>(f.esp as u32))
                    );
                }
                if f.eax != 0 && rg_low_free(p) == 0 {
                    rg_log!("BLOCKED invalid guest free {:08x}", f.eax);
                    rg_host_exit(2);
                }
                return;
            }
            6 => {
                rg_log!("getcwd buffer={:08x} size={}", f.eax as u32, f.edx as u32);
                let size = rg_host_cwd(0, std::ptr::null_mut());
                rg_log!("getcwd required={size}");
                let mut buffer = guest_ptr::<c_char>(f.eax as u32);
                let mut capacity = f.edx as u32;
                if buffer.is_null() {
                    capacity = capacity.max(size);
                    buffer = rg_low_alloc(capacity as usize).cast();
                }
                if buffer.is_null() || capacity < size || rg_host_cwd(capacity, buffer) == 0 {
                    f.eax = 0;
                    return;
                }
                rg_log!("getcwd result={}", CStr::from_ptr(buffer).to_string_lossy());
                f.eax = guest_address(buffer).into();
                return;
            }
            7 => {
                f.eax = if rg_host_chdir(guest_ptr::<c_char>(f.eax as u32)) != 0 {
                    0
                } else {
                    u32::MAX.into()
                };
                return;
            }
            5 => {
                let sig = f.eax as u32;
                if !(1..=12).contains(&sig) {
                    f.eax = 3;
                    return;
                }
                let old = SIGNAL_HANDLERS[sig as usize];
                SIGNAL_HANDLERS[sig as usize] = f.edx as u32;
                f.eax = old.into();
                return;
            }
            3 => {
                rg_log!("Native debug text display initialized");
                return;
            }
            4 => {
                let _ = io::stderr().write_all(&[f.eax as u8]);
                return;
            }
            0x1010 => {
                if rg_video_service(f) != 0 {
                    return;
                }
            }
            0x1021 => {
                if (f.eax >> 8) & 255 == 0x36 {
                    let (mut available, mut total) = (0, 0);
                    if rg_host_disk(f.edx as u32 & 255, &mut available, &mut total) == 0 {
                        f.eax = 65535;
                        return;
                    }
                    f.eax = 64;
                    f.ecx = 512;
                    f.ebx = (available / 32768).min(65535);
                    f.edx = (total / 32768).min(65535);
                    return;
                }
                if f.eax & 65535 == 0x3524 {
                    f.ebx = CRITICAL_VECTOR.into();
                    return;
                }
                if f.eax & 65535 == 0x2524 {
                    CRITICAL_VECTOR = f.edx as u32;
                    return;
                }
                if (f.eax >> 8) & 255 == 0x4c {
                    let status = f.eax as u8;
                    rg_log!(
                        "event=exit-request reason=original-dos-termination status={status} eax={:x} guest_sp={:x}",
                        f.eax,
                        f.esp
                    );
                    rg_clock_stop();
                    maiko_runtime::host::exit_cleanly(status.into());
                }
                if rg_file_service(f) != 0 {
                    return;
                }
                if (f.eax >> 8) & 255 == 0x2c {
                    let mut t = [0; 4];
                    rg_host_time(t.as_mut_ptr());
                    f.ecx = (f.ecx & 0xffff0000) | u64::from((t[0] << 8) | t[1]);
                    f.edx = (f.edx & 0xffff0000) | u64::from((t[2] << 8) | t[3]);
                    return;
                }
            }
            _ => {}
        }
        blocked(service, f);
    }
}
unsafe fn blocked(service: u32, f: &GuestFrame) -> ! {
    unsafe {
        rg_log!(
            "BLOCKED native service={service:x} eax={:08x} edx={:08x} return={:08x}",
            f.eax,
            f.edx,
            read32(guest_ptr::<u8>(f.esp as u32))
        );
        rg_dump_calls();
        rg_host_exit(2);
        std::process::abort();
    }
}
/// # Safety
/// The linker must provide the translated image and entry trampoline. Called
/// once on the initial thread with C-compatible process arguments.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rg_native_main(argc: c_int, argv: *const *const c_char) -> c_int {
    if let Err(error) = maiko_runtime::logging::initialize() {
        rg_log!("event=logging-unavailable error={error}");
    }
    unsafe {
        rg_host_setup();
        rg_crash_install();
        overlay::set_panel(overlay_panel);
        if rg_install_select(argc, argv) == 0 || rg_clock_start() == 0 {
            maiko_runtime::logging::exit("startup-installation-or-clock", 1);
            return 1;
        }
        let storage = rg_low_alloc(65536).cast::<u8>();
        let stack = rg_low_alloc(1024 * 1024);
        if storage.is_null() || stack.is_null() {
            maiko_runtime::logging::exit("startup-memory-allocation", 1);
            rg_clock_stop();
            return 1;
        }
        if rg_host_cwd(240, storage.cast()) == 0 {
            maiko_runtime::logging::exit("startup-working-directory", 1);
            rg_clock_stop();
            return 1;
        }
        let len = CStr::from_ptr(storage.cast()).to_bytes().len();
        storage
            .add(len)
            .copy_from_nonoverlapping(c"\\rgfx.exe".as_ptr().cast(), 10);
        g_1A9EBC = guest_address(storage.add(512));
        g_1A9EC0 = guest_address(storage);
        g_1A9EE9 = guest_address(storage.add(256));
        g_1A9EC8 = guest_address(stack);
        g_1A9ECC = guest_address(stack) + 1024 * 1024;
        g_1A9EB8 = g_1A9ECC;
        let mut r = GuestRegs {
            esp: g_1A9ECC,
            eax: 255,
            ..Default::default()
        };
        rg_log!("Entering original CRT initialization dispatcher");
        rg_call(guest_address(rg_init as *const ()), &mut r);
        rg_log!(
            "CRT complete argc={} argv={:08x}; entering original main",
            g_34131C,
            g_341320
        );
        rg_log!(
            "main preconditions stack={:08x} argv0={} sizes={}/{} debug={}",
            r.esp,
            ctext((guest_ptr::<u32>(g_341320)).read_unaligned()),
            object_asm_size,
            object_c_size,
            debug_type
        );
        r.eax = g_34131C;
        r.edx = g_341320;
        rg_call(guest_address(rg_main as *const ()), &mut r);
        rg_log!("Original main returned {}", r.eax);
        rg_clock_stop();
        maiko_runtime::logging::exit("original-main-return", r.eax as i32);
        r.eax as c_int
    }
}
