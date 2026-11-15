//! Miles double-buffer PCM state exposed through the original service ABI.
use crate::mixer::{Format, to_stereo16};
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
        let source = unsafe { std::slice::from_raw_parts(data.cast::<u8>(), length as usize) };
        let format = Format {
            stereo: channels == 2,
            sixteen: width == 2,
            signed: self.flags & 1 != 0,
        };
        let pcm = to_stereo16(source, format, self.rate, self.output_rate);
        if pcm.is_empty() {
            return false;
        }
        let bytes = pcm.len() * 2;
        let converted = unsafe { libc::malloc(bytes) }.cast::<u8>();
        if converted.is_null() {
            return false;
        }
        unsafe {
            converted.copy_from_nonoverlapping(pcm.as_ptr().cast(), bytes);
        }
        unsafe {
            libc::free(buffer.data.cast());
        }
        *buffer = StreamBuffer {
            data: converted,
            length: bytes as u32,
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

#[cfg(test)]
mod tests {
    use super::*;

    fn bytes(samples: &[i16]) -> Vec<u8> {
        samples.iter().flat_map(|s| s.to_le_bytes()).collect()
    }

    fn stream(format: u32, flags: u32, rate: u32) -> PcmStream {
        let mut s = PcmStream::default();
        unsafe { rg_stream_reset(&mut s) };
        (s.format, s.flags, s.rate) = (format, flags, rate);
        s
    }

    unsafe fn load(s: &mut PcmStream, index: u32, data: &[u8]) -> bool {
        unsafe { rg_stream_load(s, index, data.as_ptr().cast(), data.len() as u32) != 0 }
    }

    fn mix(s: &mut PcmStream, output: &mut [i16]) {
        unsafe { rg_stream_mix(s, output.as_mut_ptr().cast(), (output.len() * 2) as u32) };
    }

    #[test]
    fn buffers_play_in_order_with_gain_saturation_and_end() {
        let first = bytes(&[1000, -2000, 1000, -2000, 1000, -2000, 1000, -2000]);
        let second = bytes(&[3000, -4000, 3000, -4000, 3000, -4000, 3000, -4000]);
        let mut s = stream(3, 1, 44100);
        unsafe {
            assert_eq!(rg_stream_ready(&mut s), 0);
            assert!(load(&mut s, 0, &first));
            assert!(!load(&mut s, 0, &first), "a loaded buffer is not refilled");
            assert_eq!(rg_stream_ready(&mut s), 1);
            assert!(!load(&mut s, 1, &second[..3]), "partial frames are refused");
            assert!(load(&mut s, 1, &second));
            assert_eq!(rg_stream_ready(&mut s), u32::MAX);
        }
        let mut output = [10i16; 16];
        mix(&mut s, &mut output);
        for (i, value) in output.iter().enumerate() {
            let expected = match (i < 8, i % 2) {
                (true, 0) => 1010,
                (true, _) => -1990,
                (false, 0) => 3010,
                (false, _) => -3990,
            };
            assert_eq!(*value, expected, "frame value {i}");
        }
        assert_eq!((s.mixed_frames, s.nonzero_frames), (8, 8));
        unsafe {
            assert_eq!(rg_stream_status(&s), 2);
            assert_eq!(rg_stream_ready(&mut s), 0);
            assert!(rg_stream_load(&mut s, 0, std::ptr::null(), 0) != 0);
        }
        assert!(s.ended != 0);

        let mut s = stream(3, 1, 44100);
        s.pan = 0;
        unsafe {
            assert_eq!(rg_stream_ready(&mut s), 0);
            assert!(load(&mut s, 0, &second));
        }
        let mut output = [32000i16; 16];
        mix(&mut s, &mut output);
        assert_eq!(&output[..2], &[32767, 32000], "panned left and saturated");
        s.volume = 0;
        unsafe {
            assert_eq!(rg_stream_ready(&mut s), 1);
            assert!(load(&mut s, 1, &second));
        }
        let mut output = [0i16; 16];
        mix(&mut s, &mut output);
        assert!(output.iter().all(|&v| v == 0));
    }

    #[test]
    fn conversion_keeps_duration_and_silence() {
        let mut s = stream(3, 1, 44100);
        s.output_rate = 48000;
        let tone: Vec<i16> = (0..4410).flat_map(|_| [12000, -6000]).collect();
        unsafe {
            assert_eq!(rg_stream_ready(&mut s), 0);
            assert!(load(&mut s, 0, &bytes(&tone)));
        }
        assert_eq!(s.buffer[0].length, 4800 * 4);
        let converted = unsafe { std::slice::from_raw_parts(s.buffer[0].data.cast::<i16>(), 9600) };
        assert!(converted[2000] > 11000 && converted[2001] < -5000);
        unsafe { rg_stream_clear(&mut s) };
        for bits in 0..2 {
            for sign in 0..2 {
                let mut s = stream(bits, sign, 44100);
                let mut silence = vec![if sign == 1 { 0 } else { 128 }; 256];
                if bits == 1 {
                    for pair in silence.chunks_mut(2) {
                        pair.copy_from_slice(&[0, if sign == 1 { 0 } else { 128 }]);
                    }
                }
                unsafe {
                    assert_eq!(rg_stream_ready(&mut s), 0);
                    assert!(load(&mut s, 0, &silence));
                }
                let mut zeros = [0i16; 512];
                mix(&mut s, &mut zeros);
                assert!(zeros.iter().all(|&v| v == 0), "bits {bits} signed {sign}");
                unsafe {
                    assert_eq!(rg_stream_status(&s), 2);
                    rg_stream_clear(&mut s);
                }
            }
        }
        let mut s = stream(2, 0, 22050);
        s.output_rate = 22050;
        unsafe {
            assert_eq!(rg_stream_ready(&mut s), 0);
            assert!(load(&mut s, 0, &[0, 255, 128, 64]));
        }
        let mut out = [0i16; 4];
        mix(&mut s, &mut out);
        assert_eq!(out, [-32768, 32512, 0, -16384]);
    }

    #[test]
    fn partial_refills_never_starve_and_starvation_is_counted() {
        let block = bytes(&[7000; 512]);
        let mut s = stream(3, 1, 44100);
        unsafe {
            for n in 0..2 {
                assert_eq!(rg_stream_ready(&mut s), n);
                assert!(load(&mut s, n, &block));
            }
        }
        for cycle in 0..200 {
            for quarter in 0..4 {
                let mut part = [0i16; 128];
                mix(&mut s, &mut part);
                assert!(part.iter().all(|&v| v == 7000));
                if quarter < 3 {
                    assert_eq!(unsafe { rg_stream_ready(&mut s) }, u32::MAX);
                }
            }
            let next = unsafe { rg_stream_ready(&mut s) };
            assert_eq!(next, cycle & 1);
            unsafe { assert!(load(&mut s, next, &block)) };
        }
        assert!(s.mixed_frames >= 51200 && s.underrun_frames == 0);
        unsafe {
            rg_stream_clear(&mut s);
            assert_eq!(rg_stream_ready(&mut s), 0);
            assert!(load(&mut s, 0, &block));
        }
        let mut large = [0i16; 1024];
        mix(&mut s, &mut large);
        assert_eq!(s.underrun_frames, 256);
        unsafe { rg_stream_clear(&mut s) };
    }
}
