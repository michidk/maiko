//! CD music. The original plays its music from the audio tracks of the game disc through the Miles
//! `AIL_redbook_*` calls. Here those calls are services (see `native-metadata/external_procedures.sci`)
//! that read the audio tracks of a disc image through its cue sheet (the GOG `game.ins` for
//! `game.gog`, or a rip of the CD, chosen in `installed_media.rs`) and mix them into the
//! game's audio output. Without that image the open call fails and the game goes on without music, as
//! it does on a PC with no CD in the drive.
//!
//! Positions are the Miles ones: milliseconds from the start of the disc. The original asks for a
//! track's start and end (`track_info`) and then plays exactly that range (`play`), so the unit only
//! has to be consistent; it is converted to 2352-byte sectors at 75 per second.
use crate::guest::GuestFrame;
use crate::host::rg_host_readable;
use crate::rg_log;
use std::ffi::c_int;
use std::fs::File;
use std::io;
use std::path::Path;
use std::sync::Mutex;

const SECTOR: u64 = 2352;
const SECTORS_PER_SECOND: u64 = 75;
const CHUNK: usize = 256 * 1024;

// Miles status values returned by AIL_redbook_status.
const ERROR: u32 = 0;
const PLAYING: u32 = 1;
const PAUSED: u32 = 2;
const STOPPED: u32 = 3;
/// Opaque non-zero handle; the original only passes it back.
const HANDLE: u32 = 0x5244_4231;

/// The image files a cue sheet names, in order, each with the start (`INDEX 01`) of its tracks in
/// sectors from the start of that file. A single image for the whole disc (GOG `game.ins`) and a rip
/// with a file per track (Redump) are both one list of files. Tracks are numbered 1, 2, ... in order.
pub(crate) fn parse_cue(text: &str) -> Option<Vec<(String, Vec<u64>)>> {
    let mut files: Vec<(String, Vec<u64>)> = Vec::new();
    let (mut track, mut tracks) = (0, 0);
    for line in text.lines().map(str::trim) {
        let (keyword, rest) = line.split_once(char::is_whitespace).unwrap_or((line, ""));
        let rest = rest.trim();
        if keyword.eq_ignore_ascii_case("FILE") {
            let name = match rest.strip_prefix('"') {
                Some(quoted) => &quoted[..quoted.find('"')?],
                None => rest.split_whitespace().next()?,
            };
            if name.is_empty() || files.last().is_some_and(|(_, starts)| starts.is_empty()) {
                return None;
            }
            files.push((name.to_string(), Vec::new()));
        } else if keyword.eq_ignore_ascii_case("TRACK") {
            track = rest.split_whitespace().next()?.parse().ok()?;
        } else if keyword.eq_ignore_ascii_case("INDEX") && track != 0 {
            let mut words = rest.split_whitespace();
            if words.next() != Some("01") {
                continue;
            }
            let mut parts = words.next()?.split(':').map(|p| p.parse::<u64>());
            let (m, s, f) = (
                parts.next()?.ok()?,
                parts.next()?.ok()?,
                parts.next()?.ok()?,
            );
            let (_, starts) = files.last_mut()?;
            let start = (m * 60 + s) * SECTORS_PER_SECOND + f;
            if parts.next().is_some()
                || s >= 60
                || f >= 75
                || track != tracks + 1
                || starts.last().is_some_and(|&last| last >= start)
            {
                return None;
            }
            starts.push(start);
            tracks += 1;
        }
    }
    (tracks > 0 && files.iter().all(|(_, starts)| !starts.is_empty())).then_some(files)
}

fn ms_to_sector(ms: u32) -> u64 {
    u64::from(ms) * SECTORS_PER_SECOND / 1000
}

fn sector_to_ms(sector: u64) -> u32 {
    // Rounded up so that converting back lands on the same sector.
    (sector * 1000).div_ceil(SECTORS_PER_SECOND) as u32
}

/// Add `source` into `output` (both 16-bit little-endian stereo) scaled by `volume` out of 127.
fn mix_into(output: &mut [u8], source: &[u8], volume: u32) {
    for (out, src) in output.chunks_exact_mut(2).zip(source.chunks_exact(2)) {
        let sample = i32::from(i16::from_le_bytes([src[0], src[1]])) * volume as i32 / 127;
        let sum = i32::from(i16::from_le_bytes([out[0], out[1]])) + sample;
        out.copy_from_slice(&(sum.clamp(-32768, 32767) as i16).to_le_bytes());
    }
}

use crate::os::read_at;

struct Disc {
    /// Each image file with the disc sectors it holds, from first to before end.
    files: Vec<(File, u64, u64)>,
    /// Start sector of every track, then the sector after the last one.
    bounds: Vec<u64>,
}

impl Disc {
    /// The disc a cue sheet describes, its image files looked up beside it ignoring case.
    fn open(cue: &Path) -> io::Result<Self> {
        let invalid = |message: String| io::Error::new(io::ErrorKind::InvalidData, message);
        let parsed = parse_cue(&std::fs::read_to_string(cue)?)
            .ok_or_else(|| invalid("unusable cue sheet".into()))?;
        let directory = cue.parent().unwrap_or(Path::new("."));
        let (mut files, mut bounds, mut base) = (Vec::new(), Vec::new(), 0);
        for (name, starts) in parsed {
            let path = crate::installed_media::named(directory, &[&name]).ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::NotFound,
                    format!("the cue sheet names {name}, which is not beside it"),
                )
            })?;
            let file = File::open(path)?;
            let sectors = file.metadata()?.len() / SECTOR;
            if *starts.last().unwrap() >= sectors {
                return Err(invalid(format!("cue sheet is longer than {name}")));
            }
            bounds.extend(starts.iter().map(|start| base + start));
            files.push((file, base, base + sectors));
            base += sectors;
        }
        bounds.push(base);
        Ok(Self { files, bounds })
    }

    /// Read whole sectors from `sector` on, within one image file; returns the bytes read.
    fn read(&self, sector: u64, buffer: &mut [u8]) -> io::Result<usize> {
        let (file, first, end) = self
            .files
            .iter()
            .find(|(_, first, end)| (*first..*end).contains(&sector))
            .ok_or_else(|| io::Error::from(io::ErrorKind::UnexpectedEof))?;
        let length = (buffer.len() as u64).min((end - sector) * SECTOR) as usize;
        read_at(file, &mut buffer[..length], (sector - first) * SECTOR)
    }

    fn tracks(&self) -> u32 {
        self.bounds.len() as u32 - 1
    }

    /// Track number (1-based) that contains `sector`.
    fn track_at(&self, sector: u64) -> u32 {
        self.bounds[1..].partition_point(|end| *end <= sector) as u32 + 1
    }
}

#[derive(Default)]
struct Player {
    disc: Option<Disc>,
    state: Option<Playing>,
    volume: u32,
}

struct Playing {
    paused: bool,
    track: u32,
    /// Next sector to read and the sector the range ends before.
    next: u64,
    end: u64,
    chunk: Vec<u8>,
    chunk_at: usize,
}

static PLAYER: Mutex<Player> = Mutex::new(Player {
    disc: None,
    state: None,
    volume: 127,
});

fn player() -> std::sync::MutexGuard<'static, Player> {
    PLAYER.lock().unwrap_or_else(|e| e.into_inner())
}

impl Player {
    fn status(&self) -> u32 {
        match (&self.disc, &self.state) {
            (None, _) => ERROR,
            (_, None) => STOPPED,
            (_, Some(p)) if p.paused => PAUSED,
            _ => PLAYING,
        }
    }

    fn play(&mut self, start_ms: u32, end_ms: u32) -> u32 {
        let Some(disc) = &self.disc else {
            return ERROR;
        };
        let (start, end) = (
            ms_to_sector(start_ms),
            ms_to_sector(end_ms).min(*disc.bounds.last().unwrap()),
        );
        if start >= end || start < disc.bounds[0] {
            self.state = None;
            return self.status();
        }
        rg_log!(
            "CD music: playing track {} ({start_ms} ms to {end_ms} ms)",
            disc.track_at(start)
        );
        self.state = Some(Playing {
            paused: false,
            track: disc.track_at(start),
            next: start,
            end,
            chunk: Vec::new(),
            chunk_at: 0,
        });
        PLAYING
    }

    /// Fill `output` with the next audio of the playing range.
    fn mix(&mut self, output: &mut [u8]) {
        let Some(disc) = &self.disc else {
            return;
        };
        let Some(playing) = &mut self.state else {
            return;
        };
        if playing.paused {
            return;
        }
        let mut done = 0;
        while done < output.len() {
            if playing.chunk_at >= playing.chunk.len() {
                if playing.next >= playing.end {
                    self.state = None;
                    return;
                }
                let sectors = ((CHUNK as u64) / SECTOR).min(playing.end - playing.next);
                playing.chunk.resize((sectors * SECTOR) as usize, 0);
                match disc.read(playing.next, &mut playing.chunk) {
                    Ok(n) if n > 0 => {
                        playing.chunk.truncate(n / 4 * 4);
                        playing.next += (n as u64).div_ceil(SECTOR);
                    }
                    _ => {
                        self.state = None;
                        return;
                    }
                }
                playing.chunk_at = 0;
            }
            let count = (output.len() - done).min(playing.chunk.len() - playing.chunk_at);
            mix_into(
                &mut output[done..done + count],
                &playing.chunk[playing.chunk_at..playing.chunk_at + count],
                self.volume,
            );
            playing.chunk_at += count;
            done += count;
        }
    }

    fn position_ms(&self) -> u32 {
        self.state.as_ref().map_or(0, |p| {
            let unplayed = (p.chunk.len() - p.chunk_at) as u64 / SECTOR;
            sector_to_ms(p.next.saturating_sub(unplayed))
        })
    }
}

/// Mix the playing music into the 16-bit stereo output of the audio callback.
pub fn mix(output: &mut [u8]) {
    player().mix(output);
}

/// Close the disc and stop; called with the audio system.
pub fn close() {
    let mut player = player();
    player.state = None;
    player.disc = None;
}

unsafe fn put_u32(address: u32, value: u32) {
    let pointer = crate::memory::guest_ptr::<u32>(address);
    if address != 0 && rg_host_readable(pointer.cast(), 4) != 0 {
        unsafe { pointer.write_unaligned(value) };
    }
}

/// `AIL_redbook_*` entry points (cdecl, arguments on the guest stack). `service` is the call.
///
/// # Safety
/// `frame` is the live guest register frame; its stack holds the call's arguments.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rg_redbook_service(service: u32, frame: *mut GuestFrame) -> c_int {
    let Some(frame) = (unsafe { frame.as_mut() }) else {
        return 0;
    };
    let stack = crate::memory::guest_ptr::<u32>(frame.esp as u32);
    if rg_host_readable(stack.cast(), 5 * 4) == 0 {
        return 0;
    }
    let a = unsafe { std::slice::from_raw_parts(stack, 5) };
    let mut player = player();
    let known = a[1] == HANDLE;
    frame.eax = match service {
        // open_drive(drive letter): succeeds only when a disc image with its cue sheet was found.
        0 => {
            if player.disc.is_none() {
                match crate::installed_media::audio().map(|cue| Disc::open(&cue)) {
                    Some(Ok(disc)) => {
                        rg_log!("CD music: {} tracks on the disc image", disc.tracks());
                        player.disc = Some(disc);
                    }
                    Some(Err(error)) => rg_log!("CD music unavailable: {error}"),
                    None => rg_log!(
                        "CD music unavailable: no disc image with a cue sheet (the GOG game.gog with game.ins, or a rip of the CD given with --cd-image FILE.cue)"
                    ),
                }
            }
            u64::from(if player.disc.is_some() { HANDLE } else { 0 })
        }
        // close(hb)
        1 => {
            player.state = None;
            0
        }
        // status(hb)
        2 => u64::from(if known { player.status() } else { ERROR }),
        // tracks(hb)
        3 => u64::from(
            player
                .disc
                .as_ref()
                .filter(|_| known)
                .map_or(0, Disc::tracks),
        ),
        // track(hb): the playing track, 0 when nothing is playing
        4 => u64::from(match (&player.state, known) {
            (Some(p), true) if !p.paused => p.track,
            _ => 0,
        }),
        // track_info(hb, track, &start, &end), milliseconds
        5 => {
            let range = player.disc.as_ref().filter(|_| known).and_then(|d| {
                let i = (a[2] as usize).checked_sub(1)?;
                Some((*d.bounds.get(i)?, *d.bounds.get(i + 1)?))
            });
            let (start, end) = range.map_or((0, 0), |(s, e)| (sector_to_ms(s), sector_to_ms(e)));
            unsafe {
                put_u32(a[3], start);
                put_u32(a[4], end);
            }
            0
        }
        // play(hb, start, end)
        6 => u64::from(if known {
            player.play(a[2], a[3])
        } else {
            ERROR
        }),
        // stop(hb)
        7 => {
            if known {
                player.state = None;
            }
            u64::from(player.status())
        }
        // pause(hb)
        8 => {
            if let Some(p) = player.state.as_mut().filter(|_| known) {
                p.paused = true;
            }
            u64::from(player.status())
        }
        // resume(hb)
        9 => {
            if let Some(p) = player.state.as_mut().filter(|_| known) {
                p.paused = false;
            }
            u64::from(player.status())
        }
        // position(hb), milliseconds from the start of the disc while playing
        10 => u64::from(if known { player.position_ms() } else { 0 }),
        // volume(hb)
        11 => u64::from(player.volume),
        // set_volume(hb, volume 0..127): returns the new volume
        12 => {
            player.volume = a[2].min(127);
            u64::from(player.volume)
        }
        _ => return 0,
    };
    frame.flags &= !1;
    1
}

#[cfg(test)]
mod tests {
    use super::*;

    const CUE: &str = "FILE \"game.gog\" BINARY\n  TRACK 01 MODE1/2352\n    INDEX 01 00:00:00\n  TRACK 02 AUDIO\n    INDEX 01 51:59:71\n  TRACK 03 AUDIO\n    INDEX 01 57:45:00\n";

    #[test]
    fn cue_sheet_gives_start_sectors() {
        assert_eq!(
            parse_cue(CUE),
            Some(vec![(
                "game.gog".into(),
                vec![0, (51 * 60 + 59) * 75 + 71, 57 * 60 * 75 + 45 * 75]
            )])
        );
    }

    #[test]
    fn a_rip_with_a_file_per_track_lists_each_file() {
        // The Redump layout: pregap as INDEX 00, track start as INDEX 01, times per file.
        let per_track = "FILE \"Redguard (Track 1).bin\" BINARY\n  TRACK 01 MODE1/2352\n    INDEX 01 00:00:00\nfile t2.bin BINARY\n  TRACK 02 AUDIO\n    INDEX 00 00:00:00\n    INDEX 01 00:02:00\n";
        assert_eq!(
            parse_cue(per_track),
            Some(vec![
                ("Redguard (Track 1).bin".into(), vec![0]),
                ("t2.bin".into(), vec![150]),
            ])
        );
    }

    #[test]
    fn malformed_cue_sheets_are_refused() {
        for bad in [
            "",
            "TRACK 01 AUDIO\nINDEX 01 00:00:00\n",
            "FILE a BINARY\nFILE b BINARY\nTRACK 01 AUDIO\nINDEX 01 00:00:00\n",
            "FILE a BINARY\nTRACK 02 AUDIO\nINDEX 01 00:00:00\n",
            "FILE a BINARY\nTRACK 01 AUDIO\nINDEX 01 00:61:00\n",
            "FILE a BINARY\nTRACK 01 AUDIO\nINDEX 01 00:00:80\n",
            "FILE a BINARY\nTRACK 01 AUDIO\nINDEX 01 00:00:00\nTRACK 02 AUDIO\nINDEX 01 00:00:00\n",
            "FILE a BINARY\nTRACK 01 AUDIO\nINDEX 01 xx:00:00\n",
            "FILE \"\" BINARY\nTRACK 01 AUDIO\nINDEX 01 00:00:00\n",
        ] {
            assert_eq!(parse_cue(bad), None, "{bad:?}");
        }
    }

    #[test]
    fn milliseconds_and_sectors_round_trip_at_track_starts() {
        for sector in [0, 71, 233_846, 258_000] {
            assert_eq!(ms_to_sector(sector_to_ms(sector)), sector);
        }
    }

    #[test]
    fn mixing_adds_scaled_audio_and_saturates() {
        let mut out = [0u8; 8];
        out[..2].copy_from_slice(&30_000i16.to_le_bytes());
        out[2..4].copy_from_slice(&(-100i16).to_le_bytes());
        let mut src = [0u8; 8];
        src[..2].copy_from_slice(&30_000i16.to_le_bytes());
        src[2..4].copy_from_slice(&1000i16.to_le_bytes());
        mix_into(&mut out, &src, 127);
        assert_eq!(i16::from_le_bytes([out[0], out[1]]), 32767);
        assert_eq!(i16::from_le_bytes([out[2], out[3]]), 900);
        let mut half = [0u8; 4];
        mix_into(&mut half, &src[..4], 0);
        assert_eq!(half, [0; 4]);
    }

    fn disc_with(sectors: u64, starts: &[u64]) -> (tempfile::TempDir, Disc) {
        let dir = tempfile::tempdir().unwrap();
        let mut bytes = Vec::new();
        for s in 0..sectors {
            bytes.extend(std::iter::repeat_n(((s % 100) + 1) as u8, SECTOR as usize));
        }
        std::fs::write(dir.path().join("game.gog"), bytes).unwrap();
        let cue: String = std::iter::once("FILE \"game.gog\" BINARY\n".to_string())
            .chain(starts.iter().enumerate().map(|(i, s)| {
                format!(
                    "TRACK {:02} AUDIO\nINDEX 01 {:02}:{:02}:{:02}\n",
                    i + 1,
                    s / 4500,
                    s / 75 % 60,
                    s % 75
                )
            }))
            .collect();
        std::fs::write(dir.path().join("game.ins"), cue).unwrap();
        let disc = Disc::open(&dir.path().join("game.ins")).unwrap();
        (dir, disc)
    }

    #[test]
    fn playing_a_track_range_mixes_its_sectors_then_stops() {
        let (_dir, disc) = disc_with(40, &[0, 10, 30]);
        assert_eq!(disc.tracks(), 3);
        assert_eq!(disc.track_at(0), 1);
        assert_eq!(disc.track_at(10), 2);
        assert_eq!(disc.track_at(39), 3);
        let mut player = Player {
            disc: Some(disc),
            state: None,
            volume: 127,
        };
        assert_eq!(player.status(), STOPPED);
        let (start, end) = (sector_to_ms(10), sector_to_ms(30));
        assert_eq!(player.play(start, end), PLAYING);
        assert_eq!(player.state.as_ref().unwrap().track, 2);
        let mut out = vec![0u8; 4];
        player.mix(&mut out);
        assert_eq!(
            out,
            [11, 11, 11, 11],
            "first sample is sector 10's byte value"
        );
        player.pause_for_test();
        assert_eq!(player.status(), PAUSED);
        let mut quiet = vec![0u8; 4];
        player.mix(&mut quiet);
        assert_eq!(quiet, [0; 4]);
        player.state.as_mut().unwrap().paused = false;
        // Drain the 20 sectors; the range ends and the player returns to stopped.
        let mut sink = vec![0u8; 20 * SECTOR as usize];
        player.mix(&mut sink);
        assert_eq!(player.status(), STOPPED);
        assert_eq!(player.play(sector_to_ms(5), sector_to_ms(5)), STOPPED);
    }

    impl Player {
        fn pause_for_test(&mut self) {
            self.state.as_mut().unwrap().paused = true;
        }
    }

    #[test]
    fn a_cue_sheet_longer_than_the_image_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("game.gog"), vec![0; 2 * SECTOR as usize]).unwrap();
        std::fs::write(
            dir.path().join("game.ins"),
            "FILE game.gog BINARY\nTRACK 01 AUDIO\nINDEX 01 00:00:00\nTRACK 02 AUDIO\nINDEX 01 00:05:00\n",
        )
        .unwrap();
        assert!(Disc::open(&dir.path().join("game.ins")).is_err());
    }

    #[test]
    fn a_disc_split_into_files_plays_across_them_in_disc_sectors() {
        let dir = tempfile::tempdir().unwrap();
        let sectors = |count: usize, value: u8| vec![value; count * SECTOR as usize];
        std::fs::write(dir.path().join("Track 1.bin"), sectors(4, 1)).unwrap();
        std::fs::write(dir.path().join("TRACK 2.BIN"), sectors(3, 2)).unwrap();
        std::fs::write(dir.path().join("track 3.bin"), sectors(5, 3)).unwrap();
        std::fs::write(
            dir.path().join("disc.cue"),
            "FILE \"Track 1.bin\" BINARY\n TRACK 01 MODE1/2352\n  INDEX 01 00:00:00\nFILE \"Track 2.bin\" BINARY\n TRACK 02 AUDIO\n  INDEX 00 00:00:00\n  INDEX 01 00:00:01\nFILE \"Track 3.bin\" BINARY\n TRACK 03 AUDIO\n  INDEX 01 00:00:00\n",
        )
        .unwrap();
        let disc = Disc::open(&dir.path().join("disc.cue")).unwrap();
        assert_eq!(disc.bounds, [0, 5, 7, 12]);
        assert_eq!(disc.track_at(6), 2);
        let mut player = Player {
            disc: Some(disc),
            state: None,
            volume: 127,
        };
        // Track 2 runs into track 3's file; the whole range is mixed, then playback stops.
        assert_eq!(player.play(sector_to_ms(5), sector_to_ms(12)), PLAYING);
        let mut out = vec![0u8; 7 * SECTOR as usize];
        player.mix(&mut out);
        assert!(out[..2 * SECTOR as usize].iter().all(|&b| b == 2));
        assert!(out[2 * SECTOR as usize..].iter().all(|&b| b == 3));
        player.mix(&mut [0u8; 4]);
        assert_eq!(player.status(), STOPPED);
        // A file the cue sheet names but that is missing is reported.
        std::fs::remove_file(dir.path().join("track 3.bin")).unwrap();
        assert!(Disc::open(&dir.path().join("disc.cue")).is_err());
    }
}
