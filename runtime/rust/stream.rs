//! Miles double-buffer PCM state exposed through the original service ABI.
use sdl2::sys::{SDL_AudioCVT, SDL_BuildAudioCVT, SDL_ConvertAudio};
use std::ffi::{c_int, c_void};

#[repr(C)]
#[derive(Default)]
pub struct StreamBuffer {
    pub(crate) data: *mut u8,
    pub(crate) length: u32,
    pub(crate) position: u32,
    pub(crate) source_length: u32,
    pub(crate) state: c_int,
}

#[repr(C)]
#[derive(Default)]
pub struct PcmStream {
    pub(crate) buffer: [StreamBuffer; 2],
    pub(crate) current: u32,
    pub(crate) format: u32,
    pub(crate) flags: u32,
    pub(crate) rate: u32,
    pub(crate) output_rate: u32,
    pub(crate) volume: u32,
    pub(crate) pan: u32,
    pub(crate) ended: c_int,
    pub(crate) started: c_int,
    pub(crate) mixed_frames: u64,
    pub(crate) nonzero_frames: u64,
    pub(crate) underrun_frames: u64,
}

// Stream buffers move between the game and audio threads only under the
// audio state mutex. Their malloc-owned storage has no thread affinity.
unsafe impl Send for PcmStream {}

impl PcmStream {
    pub(crate) fn clear(&mut self) {
        for buffer in &mut self.buffer {
            // SDL conversion buffers use the same CRT allocator on both hosts.
            unsafe {
                libc::free(buffer.data.cast());
            }
            *buffer = StreamBuffer::default();
        }
        self.current = 0;
        self.ended = 0;
        self.started = 0;
    }

    pub(crate) unsafe fn load(&mut self, index: u32, data: *const c_void, length: u32) -> bool {
        let Some(buffer) = self.buffer.get_mut(index as usize) else {
            return false;
        };
        if buffer.state != 1 || length > 16 * 1024 * 1024 {
            return false;
        }
        let channels = if self.format & 2 != 0 { 2 } else { 1 };
        let width = if self.format & 1 != 0 { 2 } else { 1 };
        if !length.is_multiple_of(channels * width) {
            return false;
        }
        if length == 0 {
            unsafe {
                libc::free(buffer.data.cast());
            }
            buffer.data = std::ptr::null_mut();
            buffer.length = 0;
            buffer.position = 0;
            buffer.source_length = 0;
            self.ended = 1;
            return true;
        }
        if data.is_null() || self.output_rate == 0 {
            return false;
        }
        let format = match (width, self.flags & 1 != 0) {
            (2, true) => sdl2::sys::AUDIO_S16LSB,
            (2, false) => sdl2::sys::AUDIO_U16LSB,
            (_, true) => sdl2::sys::AUDIO_S8,
            (_, false) => sdl2::sys::AUDIO_U8,
        } as u16;
        let mut cvt = std::mem::MaybeUninit::<SDL_AudioCVT>::zeroed();
        if unsafe {
            SDL_BuildAudioCVT(
                cvt.as_mut_ptr(),
                format,
                channels as u8,
                self.rate as c_int,
                sdl2::sys::AUDIO_S16LSB as u16,
                2,
                self.output_rate as c_int,
            )
        } < 0
        {
            return false;
        }
        let mut cvt = unsafe { cvt.assume_init() };
        if cvt.len_mult < 1 || length > c_int::MAX as u32 / cvt.len_mult as u32 {
            return false;
        }
        let converted =
            unsafe { libc::malloc(length as usize * cvt.len_mult as usize) }.cast::<u8>();
        if converted.is_null() {
            return false;
        }
        unsafe {
            converted.copy_from_nonoverlapping(data.cast(), length as usize);
        }
        cvt.buf = converted;
        cvt.len = length as c_int;
        if unsafe { SDL_ConvertAudio(&mut cvt) } < 0 || cvt.len_cvt <= 0 || cvt.len_cvt % 4 != 0 {
            unsafe {
                libc::free(converted.cast());
            }
            return false;
        }
        unsafe {
            libc::free(buffer.data.cast());
        }
        *buffer = StreamBuffer {
            data: converted,
            length: cvt.len_cvt as u32,
            position: 0,
            source_length: length,
            state: 2,
        };
        self.started = 1;
        true
    }

    pub(crate) unsafe fn mix(&mut self, output: &mut [u8]) {
        let mut out = 0;
        while out + 4 <= output.len() {
            let buffer = &mut self.buffer[self.current as usize];
            if buffer.state != 2 {
                break;
            }
            let count = (buffer.length - buffer.position).min((output.len() - out) as u32) & !3;
            let pan = self.pan.min(127);
            let gain = [
                if pan >= 64 {
                    (127 - pan) * 255 / 63
                } else {
                    255
                },
                if pan <= 63 { pan * 255 / 63 } else { 255 },
            ];
            self.mixed_frames += u64::from(count / 4);
            let source = unsafe {
                std::slice::from_raw_parts(
                    buffer.data.add(buffer.position as usize),
                    count as usize,
                )
            };
            for (src, dst) in source
                .chunks_exact(4)
                .zip(output[out..out + count as usize].chunks_exact_mut(4))
            {
                let samples = [
                    i16::from_le_bytes([src[0], src[1]]),
                    i16::from_le_bytes([src[2], src[3]]),
                ];
                if self.volume != 0 && samples != [0, 0] {
                    self.nonzero_frames += 1;
                }
                for channel in 0..2 {
                    let offset = channel * 2;
                    let original = i16::from_le_bytes([dst[offset], dst[offset + 1]]);
                    let sample = i64::from(original)
                        + i64::from(samples[channel])
                            * i64::from(self.volume)
                            * i64::from(gain[channel])
                            / (127 * 255);
                    let sample = sample.clamp(-32768, 32767) as i16;
                    dst[offset..offset + 2].copy_from_slice(&sample.to_le_bytes());
                }
            }
            out += count as usize;
            buffer.position += count;
            if buffer.position == buffer.length {
                buffer.state = 0;
                self.current ^= 1;
            } else {
                break;
            }
        }
        if self.started != 0 && self.ended == 0 {
            self.underrun_frames += ((output.len() - out) / 4) as u64;
        }
    }
}

/// # Safety
/// `stream` points to an initialized, exclusively owned C-layout stream.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rg_stream_clear(stream: *mut PcmStream) {
    if let Some(stream) = unsafe { stream.as_mut() } {
        stream.clear();
    }
}

/// # Safety
/// `stream` points to a zero-initialized or previously initialized stream.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rg_stream_reset(stream: *mut PcmStream) {
    let Some(stream) = (unsafe { stream.as_mut() }) else {
        return;
    };
    stream.clear();
    *stream = PcmStream {
        rate: 22050,
        output_rate: 44100,
        volume: 127,
        pan: 64,
        ..PcmStream::default()
    };
}

/// # Safety
/// `stream` is an initialized stream under the audio state lock.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rg_stream_ready(stream: *mut PcmStream) -> u32 {
    let Some(stream) = (unsafe { stream.as_mut() }) else {
        return u32::MAX;
    };
    for offset in 0..2 {
        let index = (stream.current + offset) & 1;
        if stream.buffer[index as usize].state == 0 {
            stream.buffer[index as usize].state = 1;
            return index;
        }
    }
    u32::MAX
}

/// # Safety
/// `stream` is initialized and cannot be modified while inspecting its status.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rg_stream_status(stream: *const PcmStream) -> u32 {
    let Some(stream) = (unsafe { stream.as_ref() }) else {
        return 2;
    };
    if stream.buffer.iter().any(|b| b.state == 2) {
        4
    } else {
        2
    }
}

/// # Safety
/// `stream` is exclusively owned; nonempty `data` is readable for `length` bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rg_stream_load(
    stream: *mut PcmStream,
    index: u32,
    data: *const c_void,
    length: u32,
) -> c_int {
    let Some(stream) = (unsafe { stream.as_mut() }) else {
        return 0;
    };
    (unsafe { stream.load(index, data, length) }) as c_int
}

/// # Safety
/// The locked stream owns its buffers; `output` is writable for `length` bytes
/// and does not overlap those buffers or the stream state.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rg_stream_mix(stream: *mut PcmStream, output: *mut u8, length: u32) {
    let Some(stream) = (unsafe { stream.as_mut() }) else {
        return;
    };
    if length == 0 || output.is_null() {
        return;
    }
    unsafe {
        stream.mix(std::slice::from_raw_parts_mut(output, length as usize));
    }
}
