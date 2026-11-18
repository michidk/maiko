//! Miles double-buffer PCM state behind the original stream services.
use crate::mixer::{Format, to_stereo16};

/// Bytes per source frame for a Miles format: bit 0 selects 16-bit, bit 1 stereo.
pub(crate) fn frame_bytes(format: u32) -> u32 {
    (1 + (format & 1)) * (1 + (format >> 1 & 1))
}

#[derive(Default)]
pub struct StreamBuffer {
    /// Converted 16-bit stereo PCM at the output rate.
    pub(crate) data: Vec<u8>,
    pub(crate) position: u32,
    pub(crate) source_length: u32,
    /// 0 free, 1 handed to the game for filling, 2 loaded.
    pub(crate) state: u32,
}

pub struct PcmStream {
    pub(crate) buffer: [StreamBuffer; 2],
    pub(crate) current: u32,
    pub(crate) format: u32,
    pub(crate) flags: u32,
    pub(crate) rate: u32,
    pub(crate) output_rate: u32,
    pub(crate) volume: u32,
    pub(crate) pan: u32,
    pub(crate) ended: bool,
    pub(crate) started: bool,
    pub(crate) mixed_frames: u64,
    pub(crate) nonzero_frames: u64,
    pub(crate) underrun_frames: u64,
}

impl Default for PcmStream {
    fn default() -> Self {
        Self::new(crate::mixer::RATE)
    }
}

impl PcmStream {
    /// A stream in the state Miles opens one in.
    pub(crate) fn new(output_rate: u32) -> Self {
        Self {
            buffer: Default::default(),
            current: 0,
            format: 0,
            flags: 0,
            rate: 22050,
            output_rate,
            volume: 127,
            pan: 64,
            ended: false,
            started: false,
            mixed_frames: 0,
            nonzero_frames: 0,
            underrun_frames: 0,
        }
    }

    pub(crate) fn clear(&mut self) {
        self.buffer = Default::default();
        self.current = 0;
        self.ended = false;
        self.started = false;
    }

    /// Hand the next free buffer to the game, or `u32::MAX` when both are in use.
    pub(crate) fn ready(&mut self) -> u32 {
        for offset in 0..2 {
            let index = (self.current + offset) & 1;
            if self.buffer[index as usize].state == 0 {
                self.buffer[index as usize].state = 1;
                return index;
            }
        }
        u32::MAX
    }

    /// Miles status: 4 while a buffer is loaded, 2 when done.
    pub(crate) fn status(&self) -> u32 {
        if self.buffer.iter().any(|b| b.state == 2) {
            4
        } else {
            2
        }
    }

    /// Convert and load `source` into a buffer the game was handed; empty marks the end.
    pub(crate) fn load(&mut self, index: u32, source: &[u8]) -> bool {
        let Some(buffer) = self.buffer.get_mut(index as usize) else {
            return false;
        };
        let length = source.len() as u32;
        if buffer.state != 1
            || length > 16 * 1024 * 1024
            || !length.is_multiple_of(frame_bytes(self.format))
        {
            return false;
        }
        if source.is_empty() {
            *buffer = StreamBuffer {
                state: 1,
                ..StreamBuffer::default()
            };
            self.ended = true;
            return true;
        }
        if self.output_rate == 0 {
            return false;
        }
        let format = Format {
            stereo: self.format & 2 != 0,
            sixteen: self.format & 1 != 0,
            signed: self.flags & 1 != 0,
        };
        let pcm = to_stereo16(source, format, self.rate, self.output_rate);
        if pcm.is_empty() {
            return false;
        }
        *buffer = StreamBuffer {
            data: pcm.iter().flat_map(|s| s.to_le_bytes()).collect(),
            position: 0,
            source_length: length,
            state: 2,
        };
        self.started = true;
        true
    }

    pub(crate) fn mix(&mut self, output: &mut [u8]) {
        let mut out = 0;
        while out + 4 <= output.len() {
            let buffer = &mut self.buffer[self.current as usize];
            if buffer.state != 2 {
                break;
            }
            let start = buffer.position as usize;
            let count = (buffer.data.len() - start).min(output.len() - out) & !3;
            let pan = self.pan.min(127);
            let gain = [
                if pan >= 64 {
                    (127 - pan) * 255 / 63
                } else {
                    255
                },
                if pan <= 63 { pan * 255 / 63 } else { 255 },
            ];
            self.mixed_frames += (count / 4) as u64;
            for (src, dst) in buffer.data[start..start + count]
                .chunks_exact(4)
                .zip(output[out..out + count].chunks_exact_mut(4))
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
            out += count;
            buffer.position += count as u32;
            if buffer.position as usize == buffer.data.len() {
                buffer.state = 0;
                self.current ^= 1;
            } else {
                break;
            }
        }
        if self.started && !self.ended {
            self.underrun_frames += ((output.len() - out) / 4) as u64;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bytes(samples: &[i16]) -> Vec<u8> {
        samples.iter().flat_map(|s| s.to_le_bytes()).collect()
    }

    fn stream(format: u32, flags: u32, rate: u32) -> PcmStream {
        let mut s = PcmStream::new(44100);
        (s.format, s.flags, s.rate) = (format, flags, rate);
        s
    }

    fn load(s: &mut PcmStream, index: u32, data: &[u8]) -> bool {
        s.load(index, data)
    }

    fn mix(s: &mut PcmStream, output: &mut [i16]) {
        let mut bytes: Vec<u8> = output.iter().flat_map(|v| v.to_le_bytes()).collect();
        s.mix(&mut bytes);
        for (value, pair) in output.iter_mut().zip(bytes.chunks_exact(2)) {
            *value = i16::from_le_bytes([pair[0], pair[1]]);
        }
    }

    #[test]
    fn buffers_play_in_order_with_gain_saturation_and_end() {
        let first = bytes(&[1000, -2000, 1000, -2000, 1000, -2000, 1000, -2000]);
        let second = bytes(&[3000, -4000, 3000, -4000, 3000, -4000, 3000, -4000]);
        let mut s = stream(3, 1, 44100);
        {
            assert_eq!(s.ready(), 0);
            assert!(load(&mut s, 0, &first));
            assert!(!load(&mut s, 0, &first), "a loaded buffer is not refilled");
            assert_eq!(s.ready(), 1);
            assert!(!load(&mut s, 1, &second[..3]), "partial frames are refused");
            assert!(load(&mut s, 1, &second));
            assert_eq!(s.ready(), u32::MAX);
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
        {
            assert_eq!(s.status(), 2);
            assert_eq!(s.ready(), 0);
            assert!(s.load(0, &[]));
        }
        assert!(s.ended);

        let mut s = stream(3, 1, 44100);
        s.pan = 0;
        {
            assert_eq!(s.ready(), 0);
            assert!(load(&mut s, 0, &second));
        }
        let mut output = [32000i16; 16];
        mix(&mut s, &mut output);
        assert_eq!(&output[..2], &[32767, 32000], "panned left and saturated");
        s.volume = 0;
        {
            assert_eq!(s.ready(), 1);
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
        {
            assert_eq!(s.ready(), 0);
            assert!(load(&mut s, 0, &bytes(&tone)));
        }
        assert_eq!(s.buffer[0].data.len(), 4800 * 4);
        let converted: Vec<i16> = s.buffer[0]
            .data
            .chunks_exact(2)
            .map(|pair| i16::from_le_bytes([pair[0], pair[1]]))
            .collect();
        assert!(converted[2000] > 11000 && converted[2001] < -5000);
        s.clear();
        for bits in 0..2 {
            for sign in 0..2 {
                let mut s = stream(bits, sign, 44100);
                let mut silence = vec![if sign == 1 { 0 } else { 128 }; 256];
                if bits == 1 {
                    for pair in silence.chunks_mut(2) {
                        pair.copy_from_slice(&[0, if sign == 1 { 0 } else { 128 }]);
                    }
                }
                {
                    assert_eq!(s.ready(), 0);
                    assert!(load(&mut s, 0, &silence));
                }
                let mut zeros = [0i16; 512];
                mix(&mut s, &mut zeros);
                assert!(zeros.iter().all(|&v| v == 0), "bits {bits} signed {sign}");
                {
                    assert_eq!(s.status(), 2);
                    s.clear();
                }
            }
        }
        let mut s = stream(2, 0, 22050);
        s.output_rate = 22050;
        {
            assert_eq!(s.ready(), 0);
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
        {
            for n in 0..2 {
                assert_eq!(s.ready(), n);
                assert!(load(&mut s, n, &block));
            }
        }
        for cycle in 0..200 {
            for quarter in 0..4 {
                let mut part = [0i16; 128];
                mix(&mut s, &mut part);
                assert!(part.iter().all(|&v| v == 7000));
                if quarter < 3 {
                    assert_eq!(s.ready(), u32::MAX);
                }
            }
            let next = s.ready();
            assert_eq!(next, cycle & 1);
            assert!(load(&mut s, next, &block));
        }
        assert!(s.mixed_frames >= 51200 && s.underrun_frames == 0);
        {
            s.clear();
            assert_eq!(s.ready(), 0);
            assert!(load(&mut s, 0, &block));
        }
        let mut large = [0i16; 1024];
        mix(&mut s, &mut large);
        assert_eq!(s.underrun_frames, 256);
        s.clear();
    }
}
