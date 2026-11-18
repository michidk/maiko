//! Shared Miles service adapter over the sample mixer and sound output in `mixer.rs`.
use crate::guest::GuestFrame;
use crate::host::{rg_host_lock, rg_host_readable};
use crate::memory::{guest_address, guest_ptr};
use crate::memory::{rg_guest_pointer, rg_low_alloc, rg_low_free};
use crate::mixer;
use crate::rg_log;
use crate::stream::{
    PcmStream, rg_stream_load, rg_stream_ready, rg_stream_reset, rg_stream_status,
};
use std::ffi::{c_int, c_void};
use std::io::Write;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering::Relaxed};

#[derive(Default)]
struct Sample {
    handle: usize,
    user: [u32; 8],
    length: u32,
    format: u32,
}
#[derive(Default)]
struct Stream {
    handle: usize,
    user: [u32; 8],
    pcm: PcmStream,
}
#[derive(Default)]
struct Audio {
    opened: bool,
    exit_registered: bool,
    observed: bool,
    started: bool,
    driver: usize,
    samples: [Sample; 16],
    captured: usize,
    capture_failed: bool,
    output: Option<mixer::Output>,
}
static AUDIO: Mutex<Option<Audio>> = Mutex::new(None);
static STREAMS: Mutex<Option<[Stream; 8]>> = Mutex::new(None);
#[unsafe(export_name = "rg_audio_starts")]
static STARTS: AtomicU64 = AtomicU64::new(0);
static MIXED: AtomicU64 = AtomicU64::new(0);
#[unsafe(export_name = "rg_audio_nonzero")]
static NONZERO: AtomicU64 = AtomicU64::new(0);
#[unsafe(export_name = "rg_audio_stream_buffers")]
static BUFFERS: AtomicU64 = AtomicU64::new(0);
#[unsafe(export_name = "rg_audio_stream_nonzero")]
static STREAM_NONZERO: AtomicU64 = AtomicU64::new(0);
#[unsafe(export_name = "rg_audio_stream_mixed")]
static STREAM_MIXED: AtomicU64 = AtomicU64::new(0);
#[unsafe(export_name = "rg_audio_stream_underrun")]
static UNDERRUN: AtomicU64 = AtomicU64::new(0);

#[repr(C)]
pub struct AudioStats {
    started: u64,
    mixed_frames: u64,
    nonzero_frames: u64,
}

impl AudioStats {
    /// Frames of output that were not silent.
    pub fn nonzero_frames(&self) -> u64 {
        self.nonzero_frames
    }
}

/// # Safety
/// `stats` points to writable storage for the C-layout statistics.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rg_audio_stats(stats: *mut AudioStats) {
    if let Some(stats) = unsafe { stats.as_mut() } {
        *stats = AudioStats {
            started: STARTS.load(Relaxed),
            mixed_frames: MIXED.load(Relaxed),
            nonzero_frames: NONZERO.load(Relaxed),
        };
    }
}

/// Runs on the audio thread after the samples are mixed: movie streams, then CD music.
fn postmix(output: &mut [u8]) {
    {
        let mut streams = STREAMS.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(streams) = streams.as_mut() {
            for stream in streams.iter_mut().filter(|s| s.handle != 0) {
                let pcm = &mut stream.pcm;
                let before = (pcm.nonzero_frames, pcm.mixed_frames, pcm.underrun_frames);
                unsafe {
                    pcm.mix(output);
                }
                STREAM_NONZERO.fetch_add(pcm.nonzero_frames - before.0, Relaxed);
                STREAM_MIXED.fetch_add(pcm.mixed_frames - before.1, Relaxed);
                UNDERRUN.fetch_add(pcm.underrun_frames - before.2, Relaxed);
            }
        }
    }
    crate::redbook::mix(output);
    MIXED.fetch_add((output.len() / 4) as u64, Relaxed);
    NONZERO.fetch_add(
        output
            .chunks_exact(4)
            .filter(|frame| *frame != [0, 0, 0, 0])
            .count() as u64,
        Relaxed,
    );
}

impl Audio {
    fn capture(&mut self, data: &[u8]) -> bool {
        let Some(path) = std::env::var_os("RG_MOVIE_PCM_CAPTURE").filter(|p| !p.is_empty()) else {
            return true;
        };
        if data.is_empty() || self.captured == 262144 {
            return true;
        }
        if self.capture_failed {
            return false;
        }
        let count = data.len().min(262144 - self.captured);
        let result = (|| {
            let mut file = std::fs::OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(self.captured == 0)
                .append(self.captured != 0)
                .open(path)?;
            file.write_all(&data[..count])?;
            file.flush()
        })();
        if result.is_err() {
            self.capture_failed = true;
            return false;
        }
        self.captured += count;
        true
    }

    fn close(&mut self) {
        self.started = false;
        crate::redbook::close();
        if !self.opened {
            return;
        }
        // Stop the output (and its callback) before taking STREAMS or freeing any buffers.
        self.output = None;
        for sample in &mut self.samples {
            if sample.handle != 0 {
                unsafe {
                    mixer::detach(sample.handle as *mut c_void);
                    rg_low_free(sample.handle as *mut c_void);
                }
            }
            *sample = Sample::default();
        }
        {
            let mut streams = STREAMS.lock().unwrap_or_else(|e| e.into_inner());
            if let Some(streams) = streams.as_mut() {
                for stream in streams {
                    stream.pcm.clear();
                    if stream.handle != 0 {
                        unsafe {
                            rg_low_free(stream.handle as *mut c_void);
                        }
                    }
                    *stream = Stream::default();
                }
            }
        }
        if self.driver != 0 {
            unsafe {
                rg_low_free(self.driver as *mut c_void);
            }
            self.driver = 0;
        }
        self.opened = false;
        rg_log!(
            "Native movie stream buffers={} nonzero_frames={} consumed_frames={} starved_frames={}",
            BUFFERS.load(Relaxed),
            STREAM_NONZERO.load(Relaxed),
            STREAM_MIXED.load(Relaxed),
            UNDERRUN.load(Relaxed)
        );
        rg_log!(
            "Native audio closed samples_started={} mixed_frames={} nonzero_frames={}",
            STARTS.load(Relaxed),
            MIXED.load(Relaxed),
            NONZERO.load(Relaxed)
        );
    }

    unsafe fn open(&mut self, handles: &mut [u32; 16]) -> bool {
        let started = self.started;
        self.close();
        self.started = started;
        handles.fill(0);
        self.opened = true;
        self.observed = false;
        self.captured = 0;
        self.capture_failed = false;
        for counter in [
            &STARTS,
            &MIXED,
            &NONZERO,
            &BUFFERS,
            &STREAM_NONZERO,
            &STREAM_MIXED,
            &UNDERRUN,
        ] {
            counter.store(0, Relaxed);
        }
        for (sample, handle) in self.samples.iter_mut().zip(handles) {
            // Miles' sample handle size; the game only passes it back.
            let memory = rg_low_alloc(644);
            if !memory.is_null() {
                unsafe { memory.cast::<u8>().write_bytes(0, 644) };
                mixer::attach(memory);
            }
            sample.handle = memory as usize;
            if sample.handle == 0
                || unsafe { rg_guest_pointer(sample.handle as *const c_void, 644, handle) } == 0
            {
                self.close();
                return false;
            }
        }
        *STREAMS.lock().unwrap_or_else(|e| e.into_inner()) =
            Some(std::array::from_fn(|_| Stream::default()));
        self.driver = rg_low_alloc(4) as usize;
        if self.driver == 0 {
            self.close();
            return false;
        }
        unsafe {
            (self.driver as *mut u32).write(0x52474155);
            if !self.exit_registered {
                libc::atexit(rg_audio_close);
                self.exit_registered = true;
            }
        }
        let output = mixer::open(postmix);
        rg_log!(
            "Native audio device={} rate={} channels=2 backend=cpal",
            output.describe(),
            mixer::RATE
        );
        self.output = Some(output);
        true
    }

    unsafe fn service(&mut self, service: u32, frame: &mut GuestFrame) -> bool {
        match service {
            17 => {
                frame.eax = u64::from(!self.started);
                self.started = true;
                return true;
            }
            19 => {
                frame.eax = guest_address(self.driver as *const c_void).into();
                return self.opened;
            }
            _ => {}
        }
        let argc = match service {
            2 | 9 | 11 | 12 | 15 | 16 | 20 | 21 | 22 => 1,
            5 | 6 | 7 | 10 | 14 | 28 | 29 => 2,
            3 | 4 | 8 | 13 | 25 => 3,
            23 => 4,
            24 => 5,
            _ => return false,
        };
        let pointer = guest_ptr::<u32>(frame.esp as u32);
        if rg_host_readable(pointer.cast(), (argc + 1) * 4) == 0 {
            return false;
        }
        // Copy arguments before any asynchronous output or guest writes.
        let mut args = [0u32; 6];
        unsafe {
            args[..argc + 1].copy_from_slice(std::slice::from_raw_parts(pointer, argc + 1));
        }
        let a = args;
        if service == 15 {
            frame.eax = guest_address(rg_low_alloc(a[1] as usize)).into();
            return true;
        }
        if service == 16 {
            return unsafe { rg_low_free(guest_ptr::<c_void>(a[1])) } != 0;
        }
        if service == 28 || service == 29 {
            frame.eax =
                if a[2] == 0 || rg_host_readable(guest_ptr::<c_void>(a[1]), a[2] as usize) == 0 {
                    0
                } else {
                    (unsafe {
                        rg_host_lock(
                            guest_ptr::<c_void>(a[1]),
                            a[2] as usize,
                            c_int::from(service == 28),
                        )
                    }) as u64
                };
            return true;
        }
        if service == 20 {
            if !self.opened || guest_ptr::<c_void>(a[1]) as usize != self.driver {
                return false;
            }
            let mut streams = STREAMS.lock().unwrap_or_else(|e| e.into_inner());
            let Some(streams) = streams.as_mut() else {
                return false;
            };
            frame.eax = 0;
            if let Some(stream) = streams.iter_mut().find(|s| s.handle == 0) {
                stream.handle = rg_low_alloc(644) as usize;
                if stream.handle != 0 {
                    unsafe {
                        rg_stream_reset(&mut stream.pcm);
                        stream.pcm.output_rate = mixer::RATE;
                    }
                }
                frame.eax = guest_address(stream.handle as *const c_void).into();
            }
            return true;
        }
        if service == 25 {
            if !self.opened
                || guest_ptr::<c_void>(a[1]) as usize != self.driver
                || !(512..=96000).contains(&a[2])
                || a[3] > 3
            {
                return false;
            }
            let stride = (if a[3] & 1 != 0 { 2 } else { 1 }) * (if a[3] & 2 != 0 { 2 } else { 1 });
            frame.eax = (u64::from(a[2]) * stride * 2048)
                .div_ceil(u64::from(mixer::RATE))
                .next_multiple_of(256);
            return true;
        }
        if !self.opened || a[1] == 0 {
            return false;
        }
        if let Some(slot) = self
            .samples
            .iter()
            .position(|s| s.handle == guest_ptr::<c_void>(a[1]) as usize)
        {
            let sample = &mut self.samples[slot];
            let handle = sample.handle as *mut c_void;
            unsafe {
                match service {
                    2 => {
                        mixer::end(handle);
                        mixer::init(handle);
                        sample.length = 0;
                    }
                    3 => {
                        if a[2] > 3 || a[3] > 1 {
                            return false;
                        }
                        sample.format = a[2];
                        mixer::set_type(handle, a[2] as i32, a[3]);
                    }
                    4 => {
                        if a[3] > 16 * 1024 * 1024
                            || rg_host_readable(guest_ptr::<c_void>(a[2]), a[3] as usize) == 0
                        {
                            return false;
                        }
                        // With fidelity=1 the game tags 8-bit data as 16-bit; Miles drops the odd trailing byte.
                        let stride = (if sample.format & 1 != 0 { 2 } else { 1 })
                            * (if sample.format & 2 != 0 { 2 } else { 1 });
                        let length = a[3] - a[3] % stride;
                        sample.length = length;
                        mixer::set_address(handle, guest_ptr::<c_void>(a[2]), length);
                    }
                    5 => {
                        if !(512..=44000).contains(&a[2]) || mixer::status(handle) == 4 {
                            return false;
                        }
                        mixer::set_rate(handle, a[2] as i32);
                    }
                    6 => {
                        if a[2] > 127 {
                            return false;
                        }
                        mixer::set_pan(handle, a[2] as i32);
                    }
                    7 => {
                        if a[2] > i32::MAX as u32 {
                            return false;
                        }
                        mixer::set_loop_count(handle, a[2] as i32);
                    }
                    8 => return a[2] == 0 && a[3] == u32::MAX,
                    9 => {
                        let stride = (if sample.format & 1 != 0 { 2 } else { 1 })
                            * (if sample.format & 2 != 0 { 2 } else { 1 });
                        if sample.length == 0 || !sample.length.is_multiple_of(stride) {
                            return false;
                        }
                        mixer::start(handle);
                        STARTS.fetch_add(1, Relaxed);
                    }
                    10 => {
                        if a[2] > 127 {
                            return false;
                        }
                        mixer::set_volume(handle, a[2] as i32);
                    }
                    11 => {
                        frame.eax = mixer::status(handle).into();
                        if !self.observed && NONZERO.load(Relaxed) != 0 {
                            self.observed = true;
                            rg_log!(
                                "Native audio mixed nonzero_frames={} samples_started={}",
                                NONZERO.load(Relaxed),
                                STARTS.load(Relaxed)
                            );
                        }
                    }
                    12 => mixer::end(handle),
                    13 => {
                        let Some(user) = sample.user.get_mut(a[2] as usize) else {
                            return false;
                        };
                        frame.eax = (*user).into();
                        *user = a[3];
                    }
                    14 => {
                        let Some(user) = sample.user.get(a[2] as usize) else {
                            return false;
                        };
                        frame.eax = (*user).into();
                    }
                    _ => return false,
                }
            }
            return true;
        }
        unsafe { self.stream_service(service, frame, &a) }
    }

    unsafe fn stream_service(
        &mut self,
        service: u32,
        frame: &mut GuestFrame,
        a: &[u32; 6],
    ) -> bool {
        let mut streams = STREAMS.lock().unwrap_or_else(|e| e.into_inner());
        let Some(all) = streams.as_mut() else {
            return false;
        };
        let Some(slot) = all
            .iter()
            .position(|s| s.handle == guest_ptr::<c_void>(a[1]) as usize)
        else {
            return false;
        };
        if service == 23 && a[4] != 0 {
            let index = a[2] as usize;
            let stream = &mut all[slot].pcm;
            if index > 1
                || stream.buffer[index].state != 1
                || rg_host_readable(guest_ptr::<c_void>(a[3]), a[4] as usize) == 0
            {
                return false;
            }
            let mut prepared = PcmStream {
                format: stream.format,
                flags: stream.flags,
                rate: stream.rate,
                output_rate: stream.output_rate,
                ..PcmStream::default()
            };
            prepared.buffer[index].state = 1;
            drop(streams);
            if unsafe { rg_stream_load(&mut prepared, a[2], guest_ptr::<c_void>(a[3]), a[4]) } == 0
            {
                return false;
            }
            if !self.capture(unsafe {
                std::slice::from_raw_parts(guest_ptr::<u8>(a[3]), a[4] as usize)
            }) {
                prepared.clear();
                return false;
            }
            let mut streams = STREAMS.lock().unwrap_or_else(|e| e.into_inner());
            let stream = &mut streams.as_mut().unwrap()[slot].pcm;
            unsafe {
                libc::free(stream.buffer[index].data.cast());
            }
            stream.buffer[index] = std::mem::take(&mut prepared.buffer[index]);
            stream.started = 1;
            BUFFERS.fetch_add(1, Relaxed);
            return true;
        }
        let stream = &mut all[slot];
        let pcm = &mut stream.pcm;
        match service {
            2 => {
                unsafe {
                    rg_stream_reset(pcm);
                    pcm.output_rate = mixer::RATE;
                }
                stream.user.fill(0);
            }
            3 => {
                if a[2] > 3 || a[3] > 1 {
                    return false;
                }
                pcm.format = a[2];
                pcm.flags = a[3];
            }
            5 => {
                if !(512..=96000).contains(&a[2]) {
                    return false;
                }
                pcm.rate = a[2];
                rg_log!(
                    "Native movie PCM source_rate={} format={} flags={} output_rate={}",
                    pcm.rate,
                    pcm.format,
                    pcm.flags,
                    pcm.output_rate
                );
            }
            6 => {
                if a[2] > 127 {
                    return false;
                }
                pcm.pan = a[2];
            }
            10 => {
                if a[2] > 127 {
                    return false;
                }
                pcm.volume = a[2];
            }
            11 => frame.eax = unsafe { rg_stream_status(pcm) }.into(),
            12 => pcm.clear(),
            13 => {
                let Some(user) = stream.user.get_mut(a[2] as usize) else {
                    return false;
                };
                frame.eax = (*user).into();
                *user = a[3];
            }
            14 => {
                let Some(user) = stream.user.get(a[2] as usize) else {
                    return false;
                };
                frame.eax = (*user).into();
            }
            21 => {
                pcm.clear();
                unsafe {
                    rg_low_free(stream.handle as *mut c_void);
                }
                *stream = Stream::default();
            }
            22 => frame.eax = unsafe { rg_stream_ready(pcm) }.into(),
            23 => {
                if unsafe { rg_stream_load(pcm, a[2], guest_ptr::<c_void>(a[3]), a[4]) } == 0 {
                    return false;
                }
            }
            24 => {
                if a[2..]
                    .iter()
                    .any(|&p| p != 0 && rg_host_readable(guest_ptr::<c_void>(p), 4) == 0)
                {
                    return false;
                }
                let mut info = [0; 4];
                for (i, b) in pcm.buffer.iter().enumerate() {
                    info[2 * i] = if b.length == 0 {
                        0
                    } else {
                        (u64::from(b.position) * u64::from(b.source_length) / u64::from(b.length))
                            as u32
                    };
                    info[2 * i + 1] = b.source_length;
                }
                for (&address, value) in a[2..].iter().zip(info) {
                    if address != 0 {
                        unsafe {
                            (guest_ptr::<u32>(address)).write_unaligned(value);
                        }
                    }
                }
                frame.eax = u64::from(pcm.ended != 0 && unsafe { rg_stream_status(pcm) } == 2);
            }
            _ => return false,
        }
        true
    }
}

/// # Safety
/// `handles` points to sixteen writable guest handle values. Call from the game thread.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rg_audio_open(handles: *mut u32) -> c_int {
    if handles.is_null() {
        return 0;
    }
    let mut audio = AUDIO.lock().unwrap_or_else(|e| e.into_inner());
    (unsafe {
        audio
            .get_or_insert_with(Audio::default)
            .open(&mut *handles.cast::<[u32; 16]>())
    }) as c_int
}

#[unsafe(no_mangle)]
pub extern "C" fn rg_audio_close() {
    if let Some(audio) = AUDIO.lock().unwrap_or_else(|e| e.into_inner()).as_mut() {
        audio.close();
    }
}

/// # Safety
/// The guest frame and service arguments point to live mapped memory owned by the game thread.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rg_audio_service(service: u32, frame: *mut GuestFrame) -> c_int {
    let Some(frame) = (unsafe { frame.as_mut() }) else {
        return 0;
    };
    let mut audio = AUDIO.lock().unwrap_or_else(|e| e.into_inner());
    (unsafe {
        audio
            .get_or_insert_with(Audio::default)
            .service(service, frame)
    }) as c_int
}
