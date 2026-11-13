//! Checked LE loading and normalization of the two recognized executable builds.
use sha2::{Digest, Sha256};

pub const REFERENCE: &str = "844a316b5f71e3c7fadf4b012ab63c5c13aa3606a4f1504f852ac1607ee4dfdf";
pub const ALTERNATE: &str = "06d7a399cf01eec6bca3be39aaab1a39c42dc88116b6b74ae6f5f0547f7cdaf3";
const MAX_IMAGE: usize = 32 * 1024 * 1024;

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
            _ => return Err(format!("Unsupported RGFX.EXE SHA-256: {source_hash}")),
        }
        Self::parse(raw, source_hash)
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
