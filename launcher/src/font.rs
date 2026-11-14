//! Redguard's own bitmap fonts (`fonts/*.FNT` in the player's installation), for the launcher's
//! dialog titles. The format is documented in redguard-preservation's `docs/formats/FNT.md`.
use crate::artwork::Image;
use std::path::Path;

pub struct Font {
    line_height: u32,
    /// RGB, 6 bits per component as stored.
    palette: Vec<u8>,
    /// Glyphs from codepoint 32: (enabled, left, top, width, height, palette indices).
    glyphs: Vec<(bool, i32, i32, u32, u32, Vec<u8>)>,
}

/// A file in `directory` whatever the case of its name (the CD uses upper case).
pub fn named(directory: &Path, name: &str) -> Option<std::path::PathBuf> {
    std::fs::read_dir(directory)
        .ok()?
        .flatten()
        .find_map(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .eq_ignore_ascii_case(name)
                .then(|| entry.path())
        })
}

impl Font {
    /// `fonts/NAME` in the game folder.
    pub fn load(game: &Path, name: &str) -> Option<Self> {
        let fonts = named(game, "fonts")?;
        Self::parse(&std::fs::read(named(&fonts, name)?).ok()?)
    }

    pub fn parse(data: &[u8]) -> Option<Self> {
        let (mut header, mut palette, mut bitmaps) = (None, None, None);
        let mut at = 0;
        while let Some(tag) = data.get(at..at + 4) {
            if tag == b"END " {
                break;
            }
            let length = u32::from_be_bytes(data.get(at + 4..at + 8)?.try_into().ok()?) as usize;
            let payload = data.get(at + 8..at + 8 + length)?;
            match tag {
                b"FNHD" => header = Some(payload),
                b"BPAL" | b"FPAL" => palette = Some(payload),
                b"FBMP" => bitmaps = Some(payload),
                _ => {}
            }
            at += 8 + length;
        }
        let (header, palette, bitmaps) = (header?, palette?, bitmaps?);
        let u16_at = |data: &[u8], at: usize| {
            Some(u16::from_le_bytes(data.get(at..at + 2)?.try_into().ok()?))
        };
        let line_height = u32::from(u16_at(header, 0x2c)?);
        let count = usize::from(u16_at(header, 0x30)?).min(256);
        if palette.len() != 768 {
            return None;
        }
        let mut glyphs = Vec::with_capacity(count);
        let mut at = 0;
        for _ in 0..count {
            let enabled = u16_at(bitmaps, at)? != 0;
            let left = i32::from(u16_at(bitmaps, at + 2)? as i16);
            let top = i32::from(u16_at(bitmaps, at + 4)? as i16);
            let (width, height) = (
                u32::from(u16_at(bitmaps, at + 6)?),
                u32::from(u16_at(bitmaps, at + 8)?),
            );
            let size = (width * height) as usize;
            let pixels = bitmaps.get(at + 10..at + 10 + size)?.to_vec();
            glyphs.push((enabled, left, top, width, height, pixels));
            at += 10 + size;
        }
        Some(Self {
            line_height,
            palette: palette.to_vec(),
            glyphs,
        })
    }

    /// `text` drawn as the game draws it: each glyph advances by its width plus one pixel and
    /// index 0 is transparent. Characters the font lacks are skipped.
    pub fn render(&self, text: &str) -> Image {
        let glyph = |c: char| {
            (c as usize)
                .checked_sub(32)
                .and_then(|i| self.glyphs.get(i))
        };
        let width: u32 = text.chars().filter_map(glyph).map(|g| g.3 + 1).sum::<u32>() + 8;
        let height = self.line_height + 8;
        let mut rgba = vec![0u8; (width * height * 4) as usize];
        let mut x = 2i32;
        for (enabled, left, top, w, h, pixels) in text.chars().filter_map(glyph) {
            if *enabled {
                for row in 0..*h as i32 {
                    for column in 0..*w as i32 {
                        let index = pixels[(row * *w as i32 + column) as usize] as usize;
                        let (px, py) = (x + left + column, 2 + top + row);
                        if index == 0
                            || px < 0
                            || py < 0
                            || px >= width as i32
                            || py >= height as i32
                        {
                            continue;
                        }
                        let at = ((py as u32 * width + px as u32) * 4) as usize;
                        let color = &self.palette[index * 3..index * 3 + 3];
                        // Components are 6-bit; the 3dfx renderer shows c as c << 2.
                        rgba[at..at + 4].copy_from_slice(&[
                            color[0] << 2,
                            color[1] << 2,
                            color[2] << 2,
                            255,
                        ]);
                    }
                }
            }
            x += *w as i32 + 1;
        }
        Image {
            width,
            height,
            rgba,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chunk(tag: &[u8; 4], payload: &[u8]) -> Vec<u8> {
        let mut out = tag.to_vec();
        out.extend((payload.len() as u32).to_be_bytes());
        out.extend(payload);
        out
    }

    #[test]
    fn glyphs_render_with_their_palette_and_advance() {
        let mut header = vec![0u8; 56];
        header[0x2c..0x2e].copy_from_slice(&4u16.to_le_bytes());
        header[0x30..0x32].copy_from_slice(&2u16.to_le_bytes());
        let mut palette = vec![0u8; 768];
        palette[3..6].copy_from_slice(&[63, 0, 0]);
        let mut bitmaps = Vec::new();
        // Space: disabled, 2 wide. '!': one red pixel above a transparent one.
        for (enabled, width, height, pixels) in
            [(0u16, 2u16, 1u16, vec![0u8; 2]), (1, 1, 2, vec![1, 0])]
        {
            for value in [enabled, 0, 0, width, height] {
                bitmaps.extend(value.to_le_bytes());
            }
            bitmaps.extend(pixels);
        }
        let mut file = chunk(b"FNHD", &header);
        file.extend(chunk(b"BPAL", &palette));
        file.extend(chunk(b"FBMP", &bitmaps));
        file.extend(b"END ");
        let font = Font::parse(&file).unwrap();
        let image = font.render(" !");
        // Space advances 3 (width 2 + 1), so '!' lands at x 5, y 2.
        let at = ((2 * image.width + 5) * 4) as usize;
        assert_eq!(&image.rgba[at..at + 4], &[252, 0, 0, 255]);
        let below = ((3 * image.width + 5) * 4) as usize;
        assert_eq!(image.rgba[below + 3], 0, "index 0 is transparent");
        assert!(Font::parse(b"FNHD").is_none());
    }

    #[test]
    fn the_installed_title_font_loads() {
        let Some(game) = std::env::var_os("REDGUARD_DIR") else {
            return;
        };
        let font = Font::load(Path::new(&game), "FONTSEL.FNT").expect("FONTSEL.FNT");
        assert!(font.render("Game Options").width > 100);
    }
}
