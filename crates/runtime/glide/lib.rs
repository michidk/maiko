//! Translation of the DOS Glide ABI onto rustyglide, the Rust Glide renderer.
//! Guest pointer-bearing structures are read from guest memory into native values.
use maiko_runtime::memory::{guest_address, guest_ptr};
use maiko_runtime::rg_log;
use maiko_runtime::{
    guest::GuestFrame,
    input::{self, overlay, rg_input_pump, rg_input_window},
    memory::{rg_low_alloc, rg_low_free},
};
use rustyglide::texture::{self, NccData, TexInfo};
use rustyglide::types::*;
use rustyglide::{Blend, Glide, GrVertex, LockInfo, PresentFrame, Settings, Target};
use std::cell::RefCell;
use std::ffi::c_int;
use std::io::Write;
use std::time::{Duration, Instant};

#[derive(Default)]
struct Lock {
    guest: usize,
    native: Option<LockInfo>,
    bytes: usize,
    buffer: u32,
}
struct Bridge {
    glide: Glide,
    width: u32,
    height: u32,
    /// The window's drawable size the renderer was last told about.
    drawable: (u32, u32),
    locks: [Lock; 2],
    seen: [bool; 124],
    frame: u32,
    swaps: u32,
    was_flat: bool,
    last_swap: Option<Instant>,
    /// The game's last grTexClampMode arguments.
    clamp: [u32; 3],
    movie: bool,
}
/// Glide's GR_TEXTURECLAMP_CLAMP.
const CLAMP: u32 = 1;
/// Texture memory per TMU, as OpenGLide reported it to the game.
const TEXTURE_MEMORY: u32 = 16 << 20;
/// Frame buffer memory reported in MB, as OpenGLide did.
const FRAME_BUFFER_MB: u32 = 8;

/// Converts a raw Glide value, logging one that is out of range.
fn typed<T: TryFrom<u32, Error = InvalidValue>>(value: u32) -> Option<T> {
    T::try_from(value)
        .inspect_err(|e| rg_log!("Ignored Glide argument: {e}"))
        .ok()
}

impl Bridge {
    /// The game sets wrap mode once for everything, including the 256-texel tiles a movie frame is
    /// cut into. At 640x480 a tile edge lands on a pixel centre and filtering never reaches the
    /// tile's unused black side; above that, the left edge of each tile blends with it and shows a
    /// dark vertical line. Clamping while a movie plays keeps the filter inside the tile.
    fn apply_clamp(&mut self) {
        let [tmu, s, t] = self.clamp;
        let (s, t) = if self.movie { (CLAMP, CLAMP) } else { (s, t) };
        if let (Some(tmu), Some(s), Some(t)) = (typed(tmu), typed(s), typed(t)) {
            self.glide.tex_clamp_mode(tmu, s, t);
        }
    }
    /// The original waited for a 60 Hz retrace on every swap, and several animations (flames, the
    /// free-look camera) advance once per drawn frame. Faster displays or disabled vsync would run
    /// them too fast, so never swap more often than 60 times per second.
    fn pace_swap(&mut self) {
        const FRAME: Duration = Duration::from_micros(16_667);
        if let Some(last) = self.last_swap {
            let target = last + FRAME;
            if let Some(wait) = target.checked_duration_since(Instant::now()) {
                if wait > Duration::from_millis(2) {
                    std::thread::sleep(wait - Duration::from_millis(1));
                }
                while Instant::now() < target {
                    std::hint::spin_loop();
                }
            }
        }
        self.last_swap = Some(Instant::now());
    }
}
impl Default for Bridge {
    fn default() -> Self {
        // RUSTYGLIDE_LOG=FILE (or stderr) records rustyglide's and wgpu's own messages.
        rustyglide::logging::init_from_env();
        let mut settings = Settings::from_env();
        settings.core.tmu_memory = TEXTURE_MEMORY;
        settings.core.fb_memory_mb = FRAME_BUFFER_MB;
        Self {
            glide: Glide::new(settings),
            width: 640,
            height: 480,
            drawable: (0, 0),
            locks: Default::default(),
            seen: [false; 124],
            frame: 0,
            swaps: 0,
            was_flat: false,
            last_swap: None,
            clamp: [0; 3],
            movie: false,
        }
    }
}
thread_local! {static BRIDGE:RefCell<Bridge>=RefCell::new(Bridge::default());}
/// Draws the overlay at window size over the presented frame.
fn present_overlay(frame: &mut PresentFrame<'_>) {
    if overlay::is_open() {
        input::draw_overlay(overlay::Target {
            device: frame.device,
            queue: frame.queue,
            encoder: frame.encoder,
            view: frame.view,
            format: frame.format,
        });
    }
}
unsafe fn texture(address: u32) -> Option<(TexInfo, *const u8)> {
    unsafe {
        let p = guest_ptr::<u32>(address);
        let info = TexInfo {
            small_lod: typed(p.read_unaligned())?,
            large_lod: typed(p.add(1).read_unaligned())?,
            aspect: typed(p.add(2).read_unaligned())?,
            format: typed(p.add(3).read_unaligned())?,
        };
        Some((info, guest_ptr::<u8>(p.add(4).read_unaligned())))
    }
}
/// Bytes of every level of a texture as the game passes them.
fn host_size(info: &TexInfo) -> usize {
    (info.large_lod as u32..=info.small_lod as u32)
        .filter_map(Lod::from_index)
        .map(|lod| {
            (texture::level_host_texels(lod, info.aspect) * info.format.bytes_per_texel()) as usize
        })
        .sum()
}
fn dimensions(resolution: u32) -> (u32, u32) {
    Resolution::try_from(resolution)
        .ok()
        .and_then(Resolution::size)
        .unwrap_or((0, 0))
}
impl Bridge {
    /// Reads the back buffer as 565 pixels for host-side diagnostics.
    fn read_back(&mut self, mut f: impl FnMut(&LockInfo)) -> bool {
        let Some(info) = self.glide.lfb_lock(
            lock::READ_ONLY,
            Buffer::Back,
            LfbWriteMode::W565,
            Origin::UpperLeft,
            false,
        ) else {
            return false;
        };
        f(&info);
        self.glide.lfb_unlock(lock::READ_ONLY, Buffer::Back);
        true
    }
    /// Diagnostic for the reported one-frame flashes of pure fog colour: with `RG_FLASH_LOG` set,
    /// sample the back buffer at every swap and report frames that are one flat colour while the
    /// frame before was not. Reads the buffer only; never changes what is drawn.
    fn log_flat_frame(&mut self) {
        if std::env::var_os("RG_FLASH_LOG").is_none() {
            return;
        }
        let (w, h) = (self.width.max(1), self.height.max(1));
        let mut first = None;
        let mut flat = true;
        if !self.read_back(|info| {
            for i in 0..4096u32 {
                let (x, y) = (i * 7919 % w, i * 104_729 % h);
                // SAFETY: the lock buffer holds height * stride bytes.
                let p = unsafe { pixel(info, x, y) };
                if *first.get_or_insert(p) != p {
                    flat = false;
                    break;
                }
            }
        }) {
            return;
        }
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
    /// Keep this frame as the screenshot for crash and stall reports.
    fn snapshot(&mut self) {
        let (width, height) = (self.width, self.height);
        self.read_back(|info| {
            maiko_runtime::crash::store_snapshot(width, height, |x, y| {
                // SAFETY: the lock buffer holds height * stride bytes.
                unsafe { pixel(info, x, y) }
            });
        });
    }
    fn capture(&mut self) {
        let Some(directory) = std::env::var_os("RG_CAPTURE_DIR") else {
            return;
        };
        self.frame = self.frame.wrapping_add(1);
        if self.frame > 3 && !self.frame.is_multiple_of(30) {
            return;
        }
        let filename = if self.frame == 1 {
            "game-frame-first.ppm"
        } else {
            "game-frame-latest.ppm"
        };
        let (width, height, frame) = (self.width, self.height, self.frame);
        let mut result = Ok(());
        self.read_back(|info| {
            result = (|| -> std::io::Result<()> {
                let file = std::fs::File::create(std::path::Path::new(&directory).join(filename))?;
                let mut out = std::io::BufWriter::new(file);
                write!(out, "P6\n{width} {height}\n255\n")?;
                for y in 0..height {
                    for x in 0..width {
                        // SAFETY: the lock buffer holds height * stride bytes.
                        let pixel = unsafe { pixel(info, x, y) };
                        out.write_all(&[
                            ((u32::from(pixel) >> 11) * 255 / 31) as u8,
                            ((u32::from(pixel) >> 5 & 63) * 255 / 63) as u8,
                            ((u32::from(pixel) & 31) * 255 / 31) as u8,
                        ])?;
                    }
                }
                out.flush()
            })();
        });
        if result.is_ok() {
            rg_log!("Captured original frame {frame} {width}x{height}");
        }
    }
    /// Tells the renderer the window's new size after the player resized it.
    fn follow_window_size(&mut self) {
        if let Some(size) = input::drawable_size()
            && size != self.drawable
        {
            self.drawable = size;
            self.glide.resize(size.0, size.1);
        }
    }
    unsafe fn open(&mut self, a: &GuestArgs) -> u32 {
        (self.width, self.height) = dimensions(a[1]);
        if rg_input_window(self.width, self.height).is_null() {
            return 0;
        }
        let (Some((window, display, size)), Some(resolution), Some(color), Some(origin)) = (
            input::renderer_window(),
            typed::<Resolution>(a[1]),
            typed::<ColorFormat>(a[3]),
            typed::<Origin>(a[4]),
        ) else {
            return 0;
        };
        self.drawable = size;
        let target = Target::Window {
            window,
            display,
            size,
        };
        if let Err(error) = self.glide.sst_win_open(target, resolution, color, origin) {
            rg_log!(
                "Native Glide renderer failed: {error} (WGPU_BACKEND={})",
                std::env::var("WGPU_BACKEND").unwrap_or_else(|_| "automatic".into())
            );
            return 0;
        }
        self.glide.set_present_hook(Some(Box::new(present_overlay)));
        if let Some(renderer) = self.glide.renderer() {
            let info = renderer.device().adapter_info();
            rg_log!(
                "Renderer: {} via {:?}, driver {} {}",
                info.name,
                info.backend,
                info.driver,
                info.driver_info
            );
        }
        // The picture scales to any window size, so the player may resize it.
        input::set_resizable(true);
        1
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
            let g = &mut self.glide;
            match index {
                42 | 69 => {}
                45 => g.sst_win_close(),
                55 => {
                    if let Some(buffer) = typed(a[0]) {
                        g.render_buffer(buffer);
                    }
                }
                10 => g.buffer_clear(a[0], a[1] as u8, a[2] as u16),
                18 => g.color_mask(a[0] != 0, a[1] != 0),
                24 => {
                    if let Some(mode) = typed(a[0]) {
                        g.depth_buffer_mode(mode);
                    }
                }
                23 => {
                    if let Some(function) = typed(a[0]) {
                        g.depth_buffer_function(function);
                    }
                }
                25 => g.depth_mask(a[0] != 0),
                22 => g.depth_bias_level(a[0] as i16),
                86 => {
                    // --texture-filter: the game's choice, or always bilinear (1) or point (0).
                    use maiko_runtime::install::{TEXTURE_FILTER, TEXTURE_SHARP, TEXTURE_SMOOTH};
                    let (min, mag) = match TEXTURE_FILTER.load(std::sync::atomic::Ordering::Relaxed)
                    {
                        TEXTURE_SMOOTH => (1, 1),
                        TEXTURE_SHARP => (0, 0),
                        _ => (a[1], a[2]),
                    };
                    if let (Some(tmu), Some(min), Some(mag)) = (typed(a[0]), typed(min), typed(mag))
                    {
                        g.tex_filter_mode(tmu, min, mag);
                    }
                }
                77 => {
                    self.clamp = [a[0], a[1], a[2]];
                    self.apply_clamp();
                }
                90 => {
                    if let (Some(tmu), Some(mode)) = (typed(a[0]), typed(a[1])) {
                        g.tex_mipmap_mode(tmu, mode, a[2] != 0);
                    }
                }
                120 => {
                    if let (Some(tmu), Some(function)) = (typed(a[0]), typed(a[1])) {
                        g.gu_tex_combine_function(tmu, function);
                    }
                }
                101 => {
                    if let Some(function) = typed(a[0]) {
                        g.gu_color_combine_function(function);
                    }
                }
                100 => {
                    if let Some(source) = typed(a[0]) {
                        g.gu_alpha_source(source);
                    }
                }
                20 => g.constant_color(a[0]),
                5 => {
                    if let (Some(rgb_src), Some(rgb_dst), Some(alpha_src), Some(alpha_dst)) =
                        (typed(a[0]), typed(a[1]), typed(a[2]), typed(a[3]))
                    {
                        g.alpha_blend_function(Blend {
                            rgb_src,
                            rgb_dst,
                            alpha_src,
                            alpha_dst,
                        });
                    }
                }
                14 => {
                    if let Some(mode) = typed(a[0]) {
                        g.chromakey_mode(mode);
                    }
                }
                15 => g.chromakey_value(a[0]),
                27 => {
                    if let Some(mode) = typed(a[0]) {
                        g.dither_mode(mode);
                    }
                }
                37 => {
                    if let Some(mode) = typed(a[0]) {
                        g.fog_mode(mode);
                    }
                }
                36 => g.fog_color(a[0]),
                65 => {
                    // GrHwConfiguration: one Voodoo board with one TMU, as OpenGLide reports.
                    let config = guest_ptr::<u32>(a[0]);
                    for (i, value) in [1, 0, FRAME_BUFFER_MB, 2, 1, 0, 1, TEXTURE_MEMORY >> 20]
                        .into_iter()
                        .enumerate()
                    {
                        config.add(i).write_unaligned(value);
                    }
                    f.eax = 1;
                }
                75 => {
                    f.eax = self.open(&a).into();
                    rg_log!(
                        "Native Glide window {}x{} opened={}",
                        self.width,
                        self.height,
                        f.eax
                    );
                }
                39 => g.gamma_correction(f32::from_bits(a[0])),
                12 => {
                    if rg_input_pump() == 0 {
                        return false;
                    }
                    self.capture();
                    self.log_flat_frame();
                    if maiko_runtime::crash::frame_presented() {
                        self.snapshot();
                    }
                    self.follow_window_size();
                    self.glide.buffer_swap(a[0]);
                    self.pace_swap();
                }
                94 | 81 => {
                    let (Some(tmu), Some((info, data))) = (typed(a[0]), texture(a[3])) else {
                        return true;
                    };
                    if index == 94 {
                        g.tex_source(tmu, a[1], a[2], info);
                    } else {
                        let data = std::slice::from_raw_parts(data, host_size(&info));
                        g.tex_download_mipmap(tmu, a[1], a[2], info, data);
                    }
                }
                84 => {
                    let (Some(tmu), Some(table)) = (typed::<Tmu>(a[0]), typed::<TexTable>(a[1]))
                    else {
                        return true;
                    };
                    let data = guest_ptr::<u8>(a[2]);
                    match table {
                        TexTable::Palette => {
                            let entries: Vec<u32> = (0..256)
                                .map(|i| data.cast::<u32>().add(i).read_unaligned())
                                .collect();
                            g.tex_download_palette(tmu, &entries, 0);
                        }
                        TexTable::Ncc0 | TexTable::Ncc1 => {
                            let bytes = std::slice::from_raw_parts(data, 112);
                            let which = if table == TexTable::Ncc0 {
                                NccTable::Ncc0
                            } else {
                                NccTable::Ncc1
                            };
                            if let Some(ncc) = NccData::from_bytes(bytes) {
                                g.tex_download_ncc(tmu, which, &ncc);
                            }
                        }
                    }
                }
                89 | 88 => {
                    let Some(tmu) = typed(a[0]) else { return true };
                    f.eax = if index == 89 {
                        g.tex_min_address(tmu)
                    } else {
                        g.tex_max_address(tmu)
                    }
                    .into();
                }
                38 => {
                    let mut table = [0u8; FOG_TABLE_SIZE];
                    guest_ptr::<u8>(a[0])
                        .copy_to_nonoverlapping(table.as_mut_ptr(), FOG_TABLE_SIZE);
                    g.fog_table(&table);
                }
                34 => g.draw_triangle(&vertex(a[0]), &vertex(a[1]), &vertex(a[2])),
                28 => g.draw_line(&vertex(a[0]), &vertex(a[1])),
                31 => g.draw_point(&vertex(a[0])),
                49 => {
                    if a[0] > 1 || (a[2] > 2 && a[2] != 255) {
                        return false;
                    }
                    if self.locks[a[0] as usize].guest != 0 {
                        return false;
                    }
                    let info = guest_ptr::<u32>(a[5]);
                    if info.read_unaligned() != 20 {
                        return false;
                    }
                    let (Some(buffer), Some(mode), Some(origin)) =
                        (typed(a[1]), typed(a[2]), typed(a[3]))
                    else {
                        f.eax = 0;
                        return true;
                    };
                    let Some(native) = self.glide.lfb_lock(a[0], buffer, mode, origin, a[4] != 0)
                    else {
                        f.eax = 0;
                        return true;
                    };
                    let Some(bytes) = (self.height as usize).checked_mul(native.stride as usize)
                    else {
                        self.glide.lfb_unlock(a[0], buffer);
                        f.eax = 0;
                        return true;
                    };
                    let guest = rg_low_alloc(bytes) as usize;
                    if guest == 0 {
                        self.glide.lfb_unlock(a[0], buffer);
                        f.eax = 0;
                        return true;
                    }
                    (guest as *mut u8).copy_from_nonoverlapping(native.data, bytes);
                    info.cast::<[u32; 5]>().write_unaligned([
                        20,
                        guest_address(guest as *const u8),
                        native.stride,
                        native.write_mode as u32,
                        native.origin as u32,
                    ]);
                    self.locks[a[0] as usize] = Lock {
                        guest,
                        native: Some(native),
                        bytes,
                        buffer: a[1],
                    };
                    f.eax = 1;
                }
                52 => {
                    if a[0] > 1 {
                        return false;
                    }
                    let lock = std::mem::take(&mut self.locks[a[0] as usize]);
                    let Some(native) = lock
                        .native
                        .filter(|_| lock.guest != 0 && lock.buffer == a[1])
                    else {
                        self.locks[a[0] as usize] = lock;
                        return false;
                    };
                    if a[0] == 1 {
                        native
                            .data
                            .copy_from_nonoverlapping(lock.guest as *const u8, lock.bytes);
                    }
                    let buffer = typed(a[1]).unwrap_or(Buffer::Back);
                    f.eax = self.glide.lfb_unlock(a[0], buffer).into();
                    if rg_low_free(lock.guest as *mut std::ffi::c_void) == 0 {
                        return false;
                    }
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
/// A 565 pixel of a locked buffer.
///
/// # Safety
/// `(x, y)` lies inside the locked buffer.
unsafe fn pixel(info: &LockInfo, x: u32, y: u32) -> u16 {
    unsafe {
        info.data
            .add(y as usize * info.stride as usize + x as usize * 2)
            .cast::<u16>()
            .read_unaligned()
    }
}
/// A guest `GrVertex`; its layout is the same in guest and host memory.
///
/// # Safety
/// `address` points to a mapped guest vertex.
unsafe fn vertex(address: u32) -> GrVertex {
    unsafe { guest_ptr::<GrVertex>(address).read_unaligned() }
}
struct GuestArgs(*const u32);
impl std::ops::Index<usize> for GuestArgs {
    type Output = u32;
    fn index(&self, index: usize) -> &u32 {
        unsafe { &*self.0.add(index) }
    }
}
/// A movie starts (`true`) or ends; see `Bridge::apply_clamp`.
pub fn set_movie(active: bool) {
    BRIDGE.with(|bridge| {
        let mut bridge = bridge.borrow_mut();
        if bridge.movie != active {
            bridge.movie = active;
            bridge.apply_clamp();
        }
    });
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
