//! The Miles sample engine and the sound output: up to 16 game samples, converted once to 16-bit
//! stereo at 44.1 kHz, mixed with the movie streams and CD music into the system's audio device
//! (cpal). Without a device the mix still runs in real time and is discarded, so samples finish and
//! the game waits on them as usual.
use crate::Locked;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::collections::HashMap;
use std::ffi::c_void;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// The mix rate every sample and stream is converted to.
pub const RATE: u32 = 44100;

/// Raw PCM as the game hands it over.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Format {
    pub stereo: bool,
    pub sixteen: bool,
    pub signed: bool,
}

/// `data` as interleaved 16-bit stereo at `to` Hz, linearly interpolated from `from` Hz.
pub fn to_stereo16(data: &[u8], format: Format, from: u32, to: u32) -> Vec<i16> {
    let channels = if format.stereo { 2 } else { 1 };
    let width = if format.sixteen { 2 } else { 1 };
    let frames = data.len() / (channels * width);
    let sample = |frame: usize, channel: usize| -> i32 {
        let at = (frame * channels + channel.min(channels - 1)) * width;
        match (format.sixteen, format.signed) {
            (true, true) => i32::from(i16::from_le_bytes([data[at], data[at + 1]])),
            (true, false) => i32::from(u16::from_le_bytes([data[at], data[at + 1]])) - 32768,
            (false, true) => i32::from(data[at] as i8) << 8,
            (false, false) => (i32::from(data[at]) - 128) << 8,
        }
    };
    if frames == 0 || from == 0 || to == 0 {
        return Vec::new();
    }
    let out_frames = (frames as u64 * u64::from(to) / u64::from(from)) as usize;
    let mut out = Vec::with_capacity(out_frames * 2);
    for i in 0..out_frames {
        // Source position in 16.16 fixed point.
        let position = ((i as u64 * u64::from(from)) << 16) / u64::from(to);
        let (frame, fraction) = ((position >> 16) as usize, (position & 0xffff) as i32);
        let next = (frame + 1).min(frames - 1);
        for channel in 0..2 {
            let (a, b) = (sample(frame, channel), sample(next, channel));
            out.push((a + (((b - a) * fraction) >> 16)) as i16);
        }
    }
    out
}

struct Voice {
    format: Format,
    data: Option<(*const u8, usize)>,
    rate: u32,
    volume: i32,
    pan: i32,
    /// Plays left after the current one; -1 loops forever.
    loops: i32,
    pcm: Option<Arc<[i16]>>,
    position: usize,
    playing: bool,
    started: Option<Instant>,
}

// The guest pointer in `data` is only read on the game thread when a sample starts.
unsafe impl Send for Voice {}

impl Voice {
    fn new() -> Self {
        Self {
            format: Format::default(),
            data: None,
            rate: 22050,
            volume: 127,
            pan: 64,
            loops: 0,
            pcm: None,
            position: 0,
            playing: false,
            started: None,
        }
    }
}

/// Converted samples by source address, format and rate, kept with the bytes they came from so a
/// buffer the game refilled is converted again.
type Cache = HashMap<(usize, usize, Format, u32), (Vec<u8>, Arc<[i16]>)>;

struct Mixer {
    voices: HashMap<usize, Voice>,
    cache: Cache,
    master: i32,
    swap: bool,
}

static MIXER: Mutex<Option<Mixer>> = Mutex::new(None);

fn with<R>(f: impl FnOnce(&mut Mixer) -> R) -> R {
    let mut mixer = MIXER.locked();
    f(mixer.get_or_insert_with(|| Mixer {
        voices: HashMap::new(),
        cache: HashMap::new(),
        master: 127,
        swap: std::env::var_os("RG_SWAP_SOUND_CHANNELS").is_some(),
    }))
}

fn voice<R>(handle: *mut c_void, f: impl FnOnce(&mut Voice) -> R) -> Option<R> {
    with(|mixer| mixer.voices.get_mut(&(handle as usize)).map(f))
}

/// Starts tracking a sample handle the caller allocated.
pub fn attach(handle: *mut c_void) {
    with(|mixer| mixer.voices.insert(handle as usize, Voice::new()));
}

/// Stops the sample and forgets the handle.
pub fn detach(handle: *mut c_void) {
    with(|mixer| mixer.voices.remove(&(handle as usize)));
}

pub fn init(handle: *mut c_void) {
    voice(handle, |voice| *voice = Voice::new());
}

pub fn set_address(handle: *mut c_void, data: *const c_void, length: u32) {
    voice(handle, |voice| {
        voice.data = Some((data.cast(), length as usize))
    });
}

/// Miles' type: format bit 0 is 16-bit, bit 1 stereo; flags bit 0 signed.
pub fn set_type(handle: *mut c_void, format: i32, flags: u32) {
    voice(handle, |voice| {
        voice.format = Format {
            stereo: format & 2 != 0,
            sixteen: format & 1 != 0,
            signed: flags & 1 != 0,
        }
    });
}

pub fn set_rate(handle: *mut c_void, rate: i32) {
    voice(handle, |voice| voice.rate = rate.max(1) as u32);
}

pub fn set_volume(handle: *mut c_void, volume: i32) {
    voice(handle, |voice| voice.volume = volume.clamp(0, 127));
}

pub fn set_pan(handle: *mut c_void, pan: i32) {
    voice(handle, |voice| voice.pan = pan.clamp(0, 127));
}

/// Miles counts plays: 0 loops forever, n plays n times.
pub fn set_loop_count(handle: *mut c_void, count: i32) {
    voice(handle, |voice| {
        voice.loops = if count == 0 { -1 } else { count - 1 }
    });
}

pub fn set_master_volume(volume: i32) {
    with(|mixer| mixer.master = volume.clamp(0, 127));
}

/// A looping sample started again this soon, from the same data on another handle, replaces the
/// older copy.
///
/// shortcut: works around the game's `AmbientSound` script task instead of patching it. On a run
/// whose elapsed input is zero the task forgets its sample handle without stopping the sound, and
/// starts the sound again on a new handle. Elapsed input is `runs * frame scale >> 8`, so at
/// Maiko's 60+ fps (scale ~50) several of a new task's first runs read zero and each leaves
/// another copy looping (the docks' waves stacked up). The original at 12-20 fps rarely gets
/// there. Two identical loops started within this window would only sound doubled, so the old one
/// goes. Upgrade to a guest patch of `AmbientSound` that forgets the handle only on its first run
/// if a sound must legitimately loop twice from one buffer.
const RESTART_WINDOW: Duration = Duration::from_millis(250);

/// # Safety
/// The address set with [`set_address`] is readable for its length.
pub unsafe fn start(handle: *mut c_void) {
    with(|mixer| {
        let Some(voice) = mixer.voices.get_mut(&(handle as usize)) else {
            return;
        };
        voice.playing = false;
        let Some((data, length)) = voice.data.filter(|(data, _)| !data.is_null()) else {
            return;
        };
        let bytes = unsafe { std::slice::from_raw_parts(data, length) };
        let key = (data as usize, length, voice.format, voice.rate);
        let pcm = match mixer.cache.get(&key) {
            Some((source, pcm)) if source == bytes => pcm.clone(),
            _ => {
                let pcm: Arc<[i16]> = to_stereo16(bytes, voice.format, voice.rate, RATE).into();
                // shortcut: drops the whole cache when it grows past 64 entries; an LRU if starts stutter.
                if mixer.cache.len() >= 64 {
                    mixer.cache.clear();
                }
                mixer.cache.insert(key, (bytes.to_vec(), pcm.clone()));
                pcm
            }
        };
        voice.pcm = Some(pcm);
        voice.position = 0;
        voice.playing = true;
        voice.started = Some(Instant::now());
        let (source, forever) = (voice.data, voice.loops < 0);
        if forever {
            for (other_handle, other) in &mut mixer.voices {
                if *other_handle != handle as usize
                    && other.playing
                    && other.loops < 0
                    && other.data == source
                    && other
                        .started
                        .is_some_and(|at| at.elapsed() < RESTART_WINDOW)
                {
                    other.playing = false;
                }
            }
        }
    });
}

pub fn end(handle: *mut c_void) {
    voice(handle, |voice| voice.playing = false);
}

/// Miles' status: 4 playing, 2 done.
pub fn status(handle: *mut c_void) -> u32 {
    if voice(handle, |voice| voice.playing).unwrap_or(false) {
        4
    } else {
        2
    }
}

/// Adds the playing samples into `output` (16-bit stereo frames, little endian).
fn mix_voices(output: &mut [i16]) {
    with(|mixer| {
        let (master, swap) = (mixer.master, mixer.swap);
        for voice in mixer.voices.values_mut().filter(|voice| voice.playing) {
            let Some(pcm) = voice.pcm.clone() else {
                continue;
            };
            // The original's mixer scale: volume 0-128, and a 0-255 gain per side from the pan.
            let volume = (voice.volume * master * 521) >> 16;
            let side = |pan: i32| if pan >= 63 { 255 } else { (pan * 265265) >> 16 };
            let (mut left, mut right) = (side(127 - voice.pan), side(voice.pan));
            if swap {
                (left, right) = (right, left);
            }
            for frame in output.chunks_exact_mut(2) {
                if voice.position + 1 >= pcm.len() {
                    if voice.loops == 0 || pcm.len() < 2 {
                        voice.playing = false;
                        break;
                    }
                    voice.loops -= i32::from(voice.loops > 0);
                    voice.position = 0;
                }
                let gain = |value: i16, side: i32| i32::from(value) * volume / 128 * side / 255;
                frame[0] = frame[0].saturating_add(gain(pcm[voice.position], left) as i16);
                frame[1] = frame[1].saturating_add(gain(pcm[voice.position + 1], right) as i16);
                voice.position += 2;
            }
        }
    });
}

/// Mixes `frames` frames at [`RATE`]: samples, then `post` (streams and CD music) on the bytes.
fn mix(frames: usize, post: fn(&mut [u8])) -> Vec<i16> {
    let mut out = vec![0i16; frames * 2];
    mix_voices(&mut out);
    let bytes =
        unsafe { std::slice::from_raw_parts_mut(out.as_mut_ptr().cast::<u8>(), out.len() * 2) };
    post(bytes);
    out
}

/// The running output. Its thread owns the cpal stream (not `Send` on every system) or, without a
/// device, mixes in real time into nothing; dropping the output stops that thread.
pub struct Output {
    _stop: std::sync::mpsc::Sender<()>,
    name: String,
}

impl Output {
    pub fn describe(&self) -> &str {
        &self.name
    }
}

/// Starts mixing into the default device, or silently when there is none. `post` runs on the audio
/// thread after the samples are mixed.
pub fn open(post: fn(&mut [u8])) -> Output {
    use std::sync::mpsc::{RecvTimeoutError, channel};
    let (stop, stopped) = channel::<()>();
    let (opened, result) = channel::<Result<String, String>>();
    std::thread::spawn(move || match device(post) {
        Ok((stream, name)) => {
            let _ = opened.send(Ok(name));
            // The stream plays until the output is dropped.
            let _ = stopped.recv();
            drop(stream);
        }
        Err(error) => {
            let _ = opened.send(Err(error));
            let period = std::time::Duration::from_millis(10);
            while stopped.recv_timeout(period) == Err(RecvTimeoutError::Timeout) {
                mix((RATE / 100) as usize, post);
            }
        }
    });
    let name = match result.recv() {
        Ok(Ok(name)) => name,
        Ok(Err(error)) => {
            crate::rg_log!("Native audio device failed: {error}; playing silently");
            "none (silent)".to_owned()
        }
        Err(_) => "none (silent)".to_owned(),
    };
    Output { _stop: stop, name }
}

fn device(post: fn(&mut [u8])) -> Result<(cpal::Stream, String), String> {
    let host = cpal::default_host();
    let device = host.default_output_device().ok_or("no output device")?;
    let config = device.default_output_config().map_err(|e| e.to_string())?;
    let (rate, channels) = (config.sample_rate().0, usize::from(config.channels()));
    let name = format!(
        "{} ({} Hz, {} channels, {:?})",
        device.name().unwrap_or_else(|_| "unknown".into()),
        rate,
        channels,
        host.id()
    );
    // The mix runs at RATE and is resampled to the device rate; `pending` holds mixed frames not
    // yet played and `phase` the position between two of them.
    let (mut pending, mut phase) = (Vec::<i16>::new(), 0u64);
    let step = (u64::from(RATE) << 16) / u64::from(rate);
    let mut fill = move |out: &mut [f32]| {
        for frame in out.chunks_mut(channels) {
            let index = (phase >> 16) as usize;
            while pending.len() < (index + 2) * 2 {
                pending.extend(mix(512, post));
            }
            let fraction = (phase & 0xffff) as f32 / 65536.0;
            for (channel, value) in frame.iter_mut().enumerate() {
                let side = channel.min(1);
                let (a, b) = (pending[index * 2 + side], pending[index * 2 + 2 + side]);
                *value = if channel < 2 {
                    (f32::from(a) + (f32::from(b) - f32::from(a)) * fraction) / 32768.0
                } else {
                    0.0
                };
            }
            phase += step;
        }
        let used = (phase >> 16) as usize;
        pending.drain(..used * 2);
        phase &= 0xffff;
    };
    let stream = device
        .build_output_stream(
            &cpal::StreamConfig {
                channels: config.channels(),
                sample_rate: config.sample_rate(),
                buffer_size: cpal::BufferSize::Default,
            },
            move |out: &mut [f32], _| fill(out),
            |error| crate::rg_log!("Native audio stream error: {error}"),
            None,
        )
        .map_err(|e| e.to_string())?;
    stream.play().map_err(|e| e.to_string())?;
    Ok((stream, name))
}

#[cfg(test)]
mod tests {
    use super::*;

    // These tests share MIXER. Hold this across each scenario so another test's
    // mix_voices call cannot consume a finite sample before its status is checked.
    static SAMPLE_TEST: Mutex<()> = Mutex::new(());

    #[test]
    fn conversion_widens_mono_and_resamples() {
        let format = Format {
            stereo: false,
            sixteen: false,
            signed: false,
        };
        // 8-bit unsigned silence (128) and full scale, at half the output rate.
        let out = to_stereo16(&[128, 255], format, 22050, 44100);
        assert_eq!(out.len(), 8);
        assert_eq!(&out[..2], &[0, 0]);
        assert_eq!(&out[2..4], &[127 << 7, 127 << 7], "midway is interpolated");
        assert_eq!(&out[4..6], &[127 << 8, 127 << 8]);
        let stereo = Format {
            stereo: true,
            sixteen: true,
            signed: true,
        };
        assert_eq!(to_stereo16(&[1, 0, 2, 0], stereo, 44100, 44100), vec![1, 2]);
    }

    #[test]
    fn a_sample_plays_its_loops_then_reports_done() {
        let _test = SAMPLE_TEST.lock().unwrap();
        let data: Vec<u8> = vec![0, 64, 0, 64]; // two 16-bit mono frames of 16384
        let handle = 0x1000 as *mut c_void;
        attach(handle);
        set_type(handle, 1, 1);
        set_address(handle, data.as_ptr().cast(), data.len() as u32);
        set_rate(handle, RATE as i32);
        set_loop_count(handle, 2);
        unsafe { start(handle) };
        assert_eq!(status(handle), 4);
        let mut out = vec![0i16; 8];
        mix_voices(&mut out);
        assert!(
            out[..8].iter().all(|&v| v > 0),
            "two plays fill four frames: {out:?}"
        );
        mix_voices(&mut [0i16; 2]);
        assert_eq!(status(handle), 2);
        set_loop_count(handle, 0);
        unsafe { start(handle) };
        mix_voices(&mut [0i16; 64]);
        assert_eq!(status(handle), 4, "count 0 loops forever");
        end(handle);
        assert_eq!(status(handle), 2);
        detach(handle);
    }

    #[test]
    fn a_loop_restarted_on_another_handle_replaces_the_old_copy() {
        let _test = SAMPLE_TEST.lock().unwrap();
        let data: Vec<u8> = vec![0, 64, 0, 64];
        let start_loop = |handle: *mut c_void, count: i32| {
            attach(handle);
            set_type(handle, 1, 1);
            set_address(handle, data.as_ptr().cast(), data.len() as u32);
            set_loop_count(handle, count);
            unsafe { start(handle) };
        };
        let (old, new, once) = (
            0x2000 as *mut c_void,
            0x3000 as *mut c_void,
            0x4000 as *mut c_void,
        );
        start_loop(old, 0);
        start_loop(once, 1);
        start_loop(new, 0);
        assert_eq!(status(old), 2, "the forgotten loop stops");
        assert_eq!(status(new), 4);
        assert_eq!(
            status(once),
            4,
            "samples that play a set number of times are left alone"
        );
        for handle in [old, new, once] {
            detach(handle);
        }
    }
}
