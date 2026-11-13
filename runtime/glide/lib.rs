//! Translation of the DOS Glide ABI into the external OpenGLide host ABI.
//! Guest pointer-bearing structures are copied into explicit native layouts.
use maiko_runtime::memory::{guest_address, guest_ptr};
use maiko_runtime::rg_log;
use maiko_runtime::{
    guest::GuestFrame,
    input::{draw_overlay, overlay, rg_input_pump, rg_input_window},
    memory::{rg_low_alloc, rg_low_free},
};
use std::cell::RefCell;
use std::ffi::{c_int, c_void};
use std::io::Write;
#[repr(C)]
struct Texture {
    small: i32,
    large: i32,
    aspect: i32,
    format: i32,
    data: *const c_void,
}
#[repr(C)]
#[derive(Clone, Copy)]
struct NativeLfb {
    size: i32,
    pointer: *mut c_void,
    stride: u32,
    mode: i32,
    origin: i32,
}
impl Default for NativeLfb {
    fn default() -> Self {
        Self {
            size: std::mem::size_of::<Self>() as i32,
            pointer: std::ptr::null_mut(),
            stride: 0,
            mode: 0,
            origin: 0,
        }
    }
}
#[derive(Default)]
struct Lock {
    guest: usize,
    native: NativeLfb,
    bytes: usize,
    buffer: u32,
}
struct Bridge {
    width: u32,
    height: u32,
    locks: [Lock; 2],
    seen: [bool; 124],
    frame: u32,
    swaps: u32,
    was_flat: bool,
}
impl Default for Bridge {
    fn default() -> Self {
        Self {
            width: 640,
            height: 480,
            locks: Default::default(),
            seen: [false; 124],
            frame: 0,
            swaps: 0,
            was_flat: false,
        }
    }
}
thread_local! {static BRIDGE:RefCell<Bridge>=RefCell::new(Bridge::default());}
unsafe extern "C" {
    fn grBufferSwap(interval: u32);
    fn grSstIdle();
    fn grSstQueryHardware(info: *mut c_void) -> u32;
    fn grSstWinOpen(
        window: usize,
        resolution: u32,
        refresh: u32,
        color: u32,
        origin: u32,
        colors: u32,
        aux: u32,
    ) -> u32;
    fn grGammaCorrectionValue(value: f32);
    fn grTexSource(tmu: u32, address: u32, mask: u32, info: *const Texture);
    fn grTexDownloadMipMap(tmu: u32, address: u32, mask: u32, info: *const Texture);
    fn grTexDownloadTable(tmu: u32, kind: u32, data: *const c_void);
    fn grTexMinAddress(tmu: u32) -> u32;
    fn grTexMaxAddress(tmu: u32) -> u32;
    fn grFogTable(data: *const u8);
    fn grDrawTriangle(a: *const c_void, b: *const c_void, c: *const c_void);
    fn grDrawLine(a: *const c_void, b: *const c_void);
    fn grDrawPoint(a: *const c_void);
    fn grLfbLock(
        kind: u32,
        buffer: u32,
        mode: u32,
        origin: u32,
        pipeline: u32,
        info: *mut NativeLfb,
    ) -> u32;
    fn grLfbUnlock(kind: u32, buffer: u32) -> u32;
    fn grBufferClear(color: u32, alpha: u8, depth: u16);
    fn grGlideInit();
    fn grGlideShutdown();
    fn grSstSelect(a0: u32);
    fn grRenderBuffer(a0: u32);
    fn grColorMask(a0: u32, a1: u32);
    fn grDepthBufferMode(a0: u32);
    fn grDepthBufferFunction(a0: u32);
    fn grDepthMask(a0: u32);
    fn grDepthBiasLevel(a0: u32);
    fn grTexFilterMode(a0: u32, a1: u32, a2: u32);
    fn grTexClampMode(a0: u32, a1: u32, a2: u32);
    fn grTexMipMapMode(a0: u32, a1: u32, a2: u32);
    fn guTexCombineFunction(a0: u32, a1: u32);
    fn guColorCombineFunction(a0: u32);
    fn guAlphaSource(a0: u32);
    fn grConstantColorValue(a0: u32);
    fn grAlphaBlendFunction(a0: u32, a1: u32, a2: u32, a3: u32);
    fn grChromakeyMode(a0: u32);
    fn grChromakeyValue(a0: u32);
    fn grDitherMode(a0: u32);
    fn grFogMode(a0: u32);
    fn grFogColorValue(a0: u32);
}
unsafe fn texture(address: u32) -> Texture {
    unsafe {
        let p = guest_ptr::<u32>(address);
        Texture {
            small: p.read_unaligned() as i32,
            large: p.add(1).read_unaligned() as i32,
            aspect: p.add(2).read_unaligned() as i32,
            format: p.add(3).read_unaligned() as i32,
            data: guest_ptr::<c_void>(p.add(4).read_unaligned()),
        }
    }
}
fn dimensions(resolution: u32) -> (u32, u32) {
    const SIZES: [(u32, u32); 16] = [
        (320, 200),
        (320, 240),
        (400, 256),
        (512, 384),
        (640, 200),
        (640, 350),
        (640, 400),
        (640, 480),
        (800, 600),
        (960, 720),
        (856, 480),
        (512, 256),
        (1024, 768),
        (1280, 1024),
        (1600, 1200),
        (400, 300),
    ];
    SIZES.get(resolution as usize).copied().unwrap_or((0, 0))
}
impl Bridge {
    /// Diagnostic for the reported one-frame flashes of pure fog colour: with `RG_FLASH_LOG` set,
    /// sample the back buffer at every swap and report frames that are one flat colour while the
    /// frame before was not. Reads the buffer only; never changes what is drawn.
    unsafe fn log_flat_frame(&mut self) {
        unsafe {
            if std::env::var_os("RG_FLASH_LOG").is_none() {
                return;
            }
            let mut info = NativeLfb::default();
            if grLfbLock(0, 1, 0, 0, 0, &mut info) == 0 {
                return;
            }
            let (w, h) = (self.width.max(1), self.height.max(1));
            let mut first = None;
            let mut flat = true;
            for i in 0..4096u32 {
                let (x, y) = (i * 7919 % w, i * 104_729 % h);
                let p = info
                    .pointer
                    .cast::<u8>()
                    .add(y as usize * info.stride as usize + x as usize * 2)
                    .cast::<u16>()
                    .read_unaligned();
                if *first.get_or_insert(p) != p {
                    flat = false;
                    break;
                }
            }
            grLfbUnlock(0, 1);
            self.swaps = self.swaps.wrapping_add(1);
            if flat && !self.was_flat {
                rg_log!(
                    "FLASH flat frame swap={} colour={:04x}",
                    self.swaps,
                    first.unwrap_or(0)
                );
            }
            self.was_flat = flat;
        }
    }
    unsafe fn capture(&mut self) {
        unsafe {
            let Some(directory) = std::env::var_os("RG_CAPTURE_DIR") else {
                return;
            };
            self.frame = self.frame.wrapping_add(1);
            if self.frame > 3 && !self.frame.is_multiple_of(30) {
                return;
            }
            let mut info = NativeLfb::default();
            if grLfbLock(0, 1, 0, 0, 0, &mut info) == 0 {
                return;
            }
            let filename = if self.frame == 1 {
                "game-frame-first.ppm"
            } else {
                "game-frame-latest.ppm"
            };
            let result = (|| -> std::io::Result<()> {
                let file = std::fs::File::create(std::path::Path::new(&directory).join(filename))?;
                let mut out = std::io::BufWriter::new(file);
                write!(out, "P6\n{} {}\n255\n", self.width, self.height)?;
                for y in 0..self.height {
                    for x in 0..self.width {
                        let pixel = info
                            .pointer
                            .cast::<u8>()
                            .add(y as usize * info.stride as usize + x as usize * 2)
                            .cast::<u16>()
                            .read_unaligned();
                        out.write_all(&[
                            ((u32::from(pixel) >> 11) * 255 / 31) as u8,
                            ((u32::from(pixel) >> 5 & 63) * 255 / 63) as u8,
                            ((u32::from(pixel) & 31) * 255 / 31) as u8,
                        ])?;
                    }
                }
                out.flush()
            })();
            if result.is_ok() {
                rg_log!(
                    "Captured original frame {} {}x{}",
                    self.frame,
                    self.width,
                    self.height
                );
            }
            grLfbUnlock(0, 1);
        }
    }
    unsafe fn service(&mut self, index: u32, f: &mut GuestFrame) -> bool {
        unsafe {
            if index >= 124 {
                return false;
            }
            // Each import has its own argument count. Do not construct a slice
            // beyond the caller's arguments at the top of the guest stack.
            let address = guest_ptr::<u32>((f.esp as u32).wrapping_add(4));
            let a = GuestArgs(address);
            if !self.seen[index as usize] {
                rg_log!("Native Glide entry {index}");
                self.seen[index as usize] = true;
            }
            match index {
                42 => grGlideInit(),
                45 => grGlideShutdown(),
                69 => grSstSelect(a[0]),
                55 => grRenderBuffer(a[0]),
                10 => grBufferClear(a[0], a[1] as u8, a[2] as u16),
                18 => grColorMask(a[0], a[1]),
                24 => grDepthBufferMode(a[0]),
                23 => grDepthBufferFunction(a[0]),
                25 => grDepthMask(a[0]),
                22 => grDepthBiasLevel(a[0]),
                86 => grTexFilterMode(a[0], a[1], a[2]),
                77 => grTexClampMode(a[0], a[1], a[2]),
                90 => grTexMipMapMode(a[0], a[1], a[2]),
                120 => guTexCombineFunction(a[0], a[1]),
                101 => guColorCombineFunction(a[0]),
                100 => guAlphaSource(a[0]),
                20 => grConstantColorValue(a[0]),
                5 => grAlphaBlendFunction(a[0], a[1], a[2], a[3]),
                14 => grChromakeyMode(a[0]),
                15 => grChromakeyValue(a[0]),
                27 => grDitherMode(a[0]),
                37 => grFogMode(a[0]),
                36 => grFogColorValue(a[0]),
                65 => {
                    f.eax = grSstQueryHardware(guest_ptr::<c_void>(a[0])).into();
                }
                75 => {
                    (self.width, self.height) = dimensions(a[1]);
                    let window = rg_input_window(self.width, self.height);
                    if window.is_null() {
                        return false;
                    }
                    f.eax =
                        grSstWinOpen(window as usize, a[1], a[2], a[3], a[4], a[5], a[6]).into();
                    rg_log!(
                        "Native Glide window {}x{} opened={}",
                        self.width,
                        self.height,
                        f.eax
                    );
                }
                39 => {
                    grGammaCorrectionValue(f32::from_bits(a[0]));
                }
                12 => {
                    if rg_input_pump() == 0 {
                        return false;
                    }
                    self.capture();
                    self.log_flat_frame();
                    if overlay::is_open() {
                        // grBufferSwap draws the game's batched triangles; draw them first.
                        grSstIdle();
                        draw_overlay();
                    }
                    grBufferSwap(a[0]);
                }
                94 => {
                    let info = texture(a[3]);
                    grTexSource(a[0], a[1], a[2], &info);
                }
                81 => {
                    let info = texture(a[3]);
                    grTexDownloadMipMap(a[0], a[1], a[2], &info);
                }
                84 => {
                    grTexDownloadTable(a[0], a[1], guest_ptr::<c_void>(a[2]));
                }
                89 => {
                    f.eax = grTexMinAddress(a[0]).into();
                }
                88 => {
                    f.eax = grTexMaxAddress(a[0]).into();
                }
                38 => {
                    grFogTable(guest_ptr::<u8>(a[0]));
                }
                34 => {
                    grDrawTriangle(
                        guest_ptr::<c_void>(a[0]),
                        guest_ptr::<c_void>(a[1]),
                        guest_ptr::<c_void>(a[2]),
                    );
                }
                28 => {
                    grDrawLine(guest_ptr::<c_void>(a[0]), guest_ptr::<c_void>(a[1]));
                }
                31 => {
                    grDrawPoint(guest_ptr::<c_void>(a[0]));
                }
                49 => {
                    if a[0] > 1 || (a[2] > 2 && a[2] != 255) {
                        return false;
                    }
                    let lock = &mut self.locks[a[0] as usize];
                    if lock.guest != 0 {
                        return false;
                    }
                    let info = guest_ptr::<u32>(a[5]);
                    if info.read_unaligned() != 20 {
                        return false;
                    }
                    lock.native = NativeLfb::default();
                    f.eax = grLfbLock(a[0], a[1], a[2], a[3], a[4], &mut lock.native).into();
                    if f.eax == 0 {
                        return true;
                    }
                    let Some(bytes) =
                        (self.height as usize).checked_mul(lock.native.stride as usize)
                    else {
                        grLfbUnlock(a[0], a[1]);
                        f.eax = 0;
                        return true;
                    };
                    lock.bytes = bytes;
                    lock.guest = rg_low_alloc(bytes) as usize;
                    lock.buffer = a[1];
                    if lock.guest == 0 {
                        grLfbUnlock(a[0], a[1]);
                        f.eax = 0;
                        return true;
                    }
                    (lock.guest as *mut u8)
                        .copy_from_nonoverlapping(lock.native.pointer.cast(), bytes);
                    info.cast::<[u32; 5]>().write_unaligned([
                        20,
                        guest_address(lock.guest as *const u8),
                        lock.native.stride,
                        lock.native.mode as u32,
                        lock.native.origin as u32,
                    ]);
                }
                52 => {
                    if a[0] > 1 {
                        return false;
                    }
                    let lock = &mut self.locks[a[0] as usize];
                    if lock.guest == 0 || lock.buffer != a[1] {
                        return false;
                    }
                    if a[0] == 1 {
                        lock.native
                            .pointer
                            .cast::<u8>()
                            .copy_from_nonoverlapping(lock.guest as *const u8, lock.bytes);
                    }
                    f.eax = grLfbUnlock(a[0], a[1]).into();
                    if rg_low_free(lock.guest as *mut c_void) == 0 {
                        return false;
                    }
                    *lock = Lock::default();
                }
                _ => {
                    rg_log!("Unsupported Glide entry {index}");
                    return false;
                }
            }
            true
        }
    }
}
struct GuestArgs(*const u32);
impl std::ops::Index<usize> for GuestArgs {
    type Output = u32;
    fn index(&self, index: usize) -> &u32 {
        unsafe { &*self.0.add(index) }
    }
}
/// # Safety
/// `frame` and the arguments for this Glide import point to mapped guest memory.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rg_glide_service(index: u32, frame: *mut GuestFrame) -> c_int {
    let Some(frame) = (unsafe { frame.as_mut() }) else {
        return 0;
    };
    BRIDGE.with(|bridge| unsafe { bridge.borrow_mut().service(index, frame) }) as c_int
}
