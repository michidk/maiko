//! Checked LE loading and normalization of the recognized executable builds.
use regex::Regex;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

pub const REFERENCE: &str = "844a316b5f71e3c7fadf4b012ab63c5c13aa3606a4f1504f852ac1607ee4dfdf";
pub const ALTERNATE: &str = "06d7a399cf01eec6bca3be39aaab1a39c42dc88116b6b74ae6f5f0547f7cdaf3";
/// RGFX.EXE version 1.0, build 5.0, from the 1998 demo. Its code and data sit at other addresses.
pub const BUILD5: &str = "900cbfe2d586a7d80796763b1449ba050f8517b7fc7cc4742710c11e46455a31";
const MAX_IMAGE: usize = 32 * 1024 * 1024;

/// The recognized layout. Maiko's metadata names reference-image (build 1.0) addresses;
/// build 5.0 inputs are translated through build5-metadata/addresses.sci.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Build {
    Reference,
    Build5,
}

fn build5_table(name: &str) -> Result<&'static str> {
    let bytes = crate::prepare::SOURCES
        .iter()
        .find(|(path, _)| *path == format!("build5-metadata/{name}"))
        .ok_or_else(|| format!("Missing build5-metadata/{name}"))?
        .1;
    std::str::from_utf8(bytes).map_err(|_| format!("Invalid build5-metadata/{name}"))
}

fn label(field: &str) -> Result<u32> {
    field
        .strip_prefix("loc_")
        .and_then(|value| u32::from_str_radix(value, 16).ok())
        .ok_or_else(|| format!("Invalid label {field}"))
}

/// Build 1.0 address -> build 5.0 address for every address the metadata names.
pub fn build5_addresses() -> Result<BTreeMap<u32, u32>> {
    let mut map = BTreeMap::new();
    for line in build5_table("addresses.sci")?.lines() {
        if line.starts_with('#') || line.trim().is_empty() {
            continue;
        }
        let (reference, build5) = line.split_once(',').ok_or("Invalid build 5.0 address")?;
        if map.insert(label(reference)?, label(build5)?).is_some() {
            return Err(format!("Duplicate build 5.0 address {reference}"));
        }
    }
    Ok(map)
}

fn build5_sites() -> Result<BTreeMap<(u32, usize), String>> {
    let mut map = BTreeMap::new();
    for line in build5_table("sites.sci")?.lines() {
        if line.starts_with('#') || line.trim().is_empty() {
            continue;
        }
        let mut fields = line.split(',');
        let (Some(address), Some(length), Some(digest), None) =
            (fields.next(), fields.next(), fields.next(), fields.next())
        else {
            return Err("Invalid build 5.0 site".into());
        };
        let length = length
            .parse()
            .map_err(|_| "Invalid build 5.0 site length")?;
        map.insert((label(address)?, length), digest.to_owned());
    }
    Ok(map)
}

pub type Result<T> = std::result::Result<T, String>;

pub fn hash(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

pub struct Object {
    pub base: u32,
    pub flags: u32,
    pub data: Vec<u8>,
}

pub struct Image {
    pub source_hash: String,
    pub raw: Vec<u8>,
    pub objects: Vec<Object>,
    pub build: Build,
    addresses: BTreeMap<u32, u32>,
    sites: BTreeMap<(u32, usize), String>,
    table: usize,
}

fn range(bytes: &[u8], offset: usize, len: usize) -> Result<&[u8]> {
    let end = offset.checked_add(len).ok_or("Image offset overflow")?;
    bytes
        .get(offset..end)
        .ok_or_else(|| "Truncated LE image".into())
}

fn u32_at(bytes: &[u8], offset: usize) -> Result<u32> {
    let value: [u8; 4] = range(bytes, offset, 4)?
        .try_into()
        .map_err(|_| "Invalid u32")?;
    Ok(u32::from_le_bytes(value))
}

impl Image {
    pub fn load(bytes: &[u8]) -> Result<Self> {
        if bytes.len() > MAX_IMAGE {
            return Err("RGFX.EXE exceeds the 32 MiB input limit".into());
        }
        let source_hash = hash(bytes);
        let mut raw = bytes.to_vec();
        match source_hash.as_str() {
            REFERENCE => {}
            ALTERNATE => {
                // Original: the alternate recognized image has EB and EB73 at
                // these file offsets. Replacement: reference-image 74 and B902.
                // Effect: both inputs use the same checked translation path;
                // the runtime still selects behavior by the installed image hash.
                // Why: the translator's metadata describes the reference build.
                // Evidence: metadata/image_variants.json; final full-file SHA-256.
                for (offset, original, replacement) in [
                    (0x9a041, &[0xeb][..], &[0x74][..]),
                    (0xf256b, &[0xeb, 0x73][..], &[0xb9, 0x02][..]),
                ] {
                    if range(&raw, offset, original.len())? != original {
                        return Err("Alternate image normalization bytes changed".into());
                    }
                    raw[offset..offset + replacement.len()].copy_from_slice(replacement);
                }
                if hash(&raw) != REFERENCE {
                    return Err("Normalized image does not match the reference SHA-256".into());
                }
            }
            BUILD5 => {
                let mut image = Self::parse(raw, source_hash)?;
                image.build = Build::Build5;
                image.addresses = build5_addresses()?;
                image.sites = build5_sites()?;
                return Ok(image);
            }
            _ => return Err(format!("Unsupported RGFX.EXE SHA-256: {source_hash}")),
        }
        Self::parse(raw, source_hash)
    }

    /// This image's address for a reference-image address named by the metadata.
    pub fn address(&self, reference: u32) -> Result<u32> {
        match self.build {
            Build::Reference => Ok(reference),
            Build::Build5 => self
                .addresses
                .get(&reference)
                .copied()
                .ok_or_else(|| format!("No build 5.0 address for {reference:#x}")),
        }
    }

    /// Checks the bytes of a hash-guarded site named by its reference-image address and SHA-256,
    /// and returns the site's address in this image. Build 5.0 sites carry their own hashes.
    pub fn check(&self, reference: u32, length: usize, digest: &str) -> Result<u32> {
        let address = self.address(reference)?;
        let expected = match self.build {
            Build::Reference => digest,
            Build::Build5 => self
                .sites
                .get(&(reference, length))
                .ok_or_else(|| format!("No build 5.0 hash for {reference:#x}"))?,
        };
        if hash(self.at(address, length)?) != expected {
            return Err(format!("Original bytes changed at {address:#x}"));
        }
        Ok(address)
    }

    fn in_image(&self, address: u32) -> bool {
        self.objects.iter().any(|object| {
            address
                .checked_sub(object.base)
                .is_some_and(|offset| (offset as usize) < object.data.len())
        })
    }

    /// Rewrites every loc_ label that names a reference-image address to this image's address.
    pub fn relabel(&self, text: &str) -> Result<String> {
        if self.build == Build::Reference {
            return Ok(text.to_owned());
        }
        let pattern = Regex::new(r"\bloc_([0-9A-F]{5,6})\b").map_err(|e| e.to_string())?;
        let mut output = String::with_capacity(text.len());
        let mut last = 0;
        for capture in pattern.captures_iter(text) {
            let whole = capture.get(0).ok_or("Invalid label match")?;
            let reference = u32::from_str_radix(&capture[1], 16).map_err(|e| e.to_string())?;
            output.push_str(&text[last..whole.start()]);
            if self.in_image(reference) {
                output.push_str(&format!("loc_{:X}", self.address(reference)?));
            } else {
                output.push_str(whole.as_str());
            }
            last = whole.end();
        }
        output.push_str(&text[last..]);
        Ok(output)
    }

    /// Rewrites a global_aliases.sci table. Exported names stay those of the reference image, so
    /// the runtime links unchanged. A moved loc_ name is emitted as a placeholder that
    /// [`rename_moved_aliases`] restores after translation.
    pub fn aliases(&self, text: &str) -> Result<String> {
        if self.build == Build::Reference {
            return Ok(text.to_owned());
        }
        let mut output = String::new();
        for line in text.lines() {
            match line.split_once(',') {
                Some((address, name)) if !line.starts_with('#') => {
                    let reference = label(address)?;
                    let moved = self.address(reference)?;
                    let name = if name == address && moved != reference {
                        format!("{MOVED_PREFIX}{name}")
                    } else {
                        name.to_owned()
                    };
                    output.push_str(&format!("loc_{moved:X},{name}\n"));
                }
                _ => {
                    output.push_str(line);
                    output.push('\n');
                }
            }
        }
        Ok(output)
    }

    fn parse(raw: Vec<u8>, source_hash: String) -> Result<Self> {
        if range(&raw, 0, 2)? != b"MZ" {
            return Err("Expected a DOS executable header".into());
        }
        let header = u32_at(&raw, 0x3c)? as usize;
        if range(&raw, header, 4)? != b"LE\0\0" {
            return Err("Expected little-endian LE executable".into());
        }
        let table = header
            .checked_add(u32_at(&raw, header + 64)? as usize)
            .ok_or("Object table overflow")?;
        let count = u32_at(&raw, header + 68)? as usize;
        if count == 0 || count > 64 {
            return Err("Unsupported LE object count".into());
        }
        let page_size = u32_at(&raw, header + 40)? as usize;
        if page_size == 0 || page_size > 65536 {
            return Err("Unsupported LE page size".into());
        }
        let page_map = header
            .checked_add(u32_at(&raw, header + 72)? as usize)
            .ok_or("Page map overflow")?;
        let pages = u32_at(&raw, header + 128)? as usize;
        let mut objects = Vec::with_capacity(count);
        let mut total = 0usize;
        for index in 0..count {
            let entry = table + index * 24;
            let size = u32_at(&raw, entry)? as usize;
            total = total.checked_add(size).ok_or("Image size overflow")?;
            if total > MAX_IMAGE {
                return Err("LE image allocation exceeds 32 MiB".into());
            }
            let base = u32_at(&raw, entry + 4)?;
            let flags = u32_at(&raw, entry + 8)?;
            let first = u32_at(&raw, entry + 12)? as usize;
            let count = u32_at(&raw, entry + 16)? as usize;
            if count > size.div_ceil(page_size) || (count > 0 && first == 0) {
                return Err("Invalid LE object page count".into());
            }
            let mut data = vec![0; size];
            for page in 0..count {
                let index = first - 1 + page;
                let mapped = range(&raw, page_map + index * 4, 4)?;
                let physical = (usize::from(mapped[0]) << 16)
                    | (usize::from(mapped[1]) << 8)
                    | usize::from(mapped[2]);
                if physical == 0 || mapped[3] != 0 {
                    return Err("Unsupported LE page map entry".into());
                }
                let start = (physical - 1)
                    .checked_mul(page_size)
                    .and_then(|n| n.checked_add(pages))
                    .ok_or("Page offset overflow")?;
                let destination = page * page_size;
                let len = page_size.min(size - destination);
                data[destination..destination + len].copy_from_slice(range(&raw, start, len)?);
            }
            objects.push(Object { base, flags, data });
        }
        Ok(Self {
            source_hash,
            raw,
            objects,
            build: Build::Reference,
            addresses: BTreeMap::new(),
            sites: BTreeMap::new(),
            table,
        })
    }

    pub fn at(&self, address: u32, len: usize) -> Result<&[u8]> {
        for object in &self.objects {
            if let Some(offset) = address.checked_sub(object.base)
                && let Some(bytes) = object
                    .data
                    .get(offset as usize..)
                    .and_then(|bytes| bytes.get(..len))
            {
                return Ok(bytes);
            }
        }
        Err(format!("Address outside image: {address:#x}"))
    }

    pub fn translator_input(&self) -> Result<Vec<u8>> {
        let mut input = self.raw.clone();
        for number in [5, 7] {
            let offset = self.table + (number - 1) * 24 + 8;
            if u32_at(&input, offset)? != 0x2043 {
                return Err(format!("Object {number} classification changed"));
            }
            input[offset..offset + 4].copy_from_slice(&0x2047u32.to_le_bytes());
        }
        Ok(input)
    }
}

const MOVED_PREFIX: &str = "rg_reference_";

/// Exported loc_ names (self-aliases in a global_aliases.sci table) whose address moved in build 5.0.
pub fn moved_aliases(build: Build) -> Result<Vec<String>> {
    if build == Build::Reference {
        return Ok(Vec::new());
    }
    let addresses = build5_addresses()?;
    let mut names = std::collections::BTreeSet::new();
    for (path, bytes) in crate::prepare::SOURCES {
        if !path.ends_with("/global_aliases.sci") {
            continue;
        }
        for line in std::str::from_utf8(bytes)
            .map_err(|_| "Invalid alias table")?
            .lines()
        {
            if let Some((address, name)) = line.split_once(',')
                && name == address
                && addresses.get(&label(address)?) != Some(&label(address)?)
            {
                names.insert(name.to_owned());
            }
        }
    }
    Ok(names.into_iter().collect())
}

/// Gives exported loc_ names that moved in build 5.0 their reference-image names again.
/// Build 5.0 can also use the old address as a label of its own; that label becomes loc_*_build5.
pub fn rename_moved_aliases(names: &[String], text: &str) -> Result<String> {
    let mut text = text.to_owned();
    for name in names {
        let own = Regex::new(&format!(r"\b{name}\b")).map_err(|e| e.to_string())?;
        if own.is_match(&text) {
            text = own
                .replace_all(&text, format!("{name}_build5").as_str())
                .into_owned();
        }
        text = text.replace(&format!("{MOVED_PREFIX}{name}"), name);
    }
    Ok(text)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_unknown_input() {
        assert!(
            Image::load(b"MZ")
                .err()
                .unwrap()
                .contains("Unsupported RGFX.EXE")
        );
    }
    #[test]
    fn build5_tables_parse() {
        let addresses = build5_addresses().unwrap();
        assert!(addresses.len() > 1000);
        assert!(build5_sites().unwrap().len() > 60);
    }
    #[test]
    fn restores_moved_alias_names_after_translation() {
        let text = "loc_1A3865:\nrg_reference_loc_1A3865:\nmov eax, [loc_1A3865]\nmov eax, [rg_reference_loc_1A3865]\n";
        let names = ["loc_1A3865".to_owned()];
        assert_eq!(
            rename_moved_aliases(&names, text).unwrap(),
            "loc_1A3865_build5:\nloc_1A3865:\nmov eax, [loc_1A3865_build5]\nmov eax, [loc_1A3865]\n"
        );
        assert!(moved_aliases(Build::Reference).unwrap().is_empty());
        assert!(
            moved_aliases(Build::Build5)
                .unwrap()
                .contains(&"loc_1A3865".to_owned())
        );
    }
    #[test]
    fn rejects_truncated_headers_without_panicking() {
        for size in 0..200 {
            assert!(Image::parse(vec![0; size], String::new()).is_err());
        }
    }
    #[test]
    fn rejects_oversized_input_before_hashing() {
        assert!(
            Image::load(&vec![0; MAX_IMAGE + 1])
                .err()
                .unwrap()
                .contains("limit")
        );
    }
}
