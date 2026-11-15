//! Deterministic universal executable assembly, shared by kit acceptance and WASM.
use crate::image::Result;

fn word(bytes: &[u8], offset: usize) -> Result<u32> {
    Ok(u32::from_le_bytes(
        bytes
            .get(offset..offset + 4)
            .ok_or("Truncated Mach-O header")?
            .try_into()
            .map_err(|_| "Invalid Mach-O header")?,
    ))
}

fn thin(bytes: &[u8], cpu: u32, subtype: u32) -> Result<()> {
    if bytes.len() > 256 * 1024 * 1024
        || word(bytes, 0)? != 0xfeedfacf
        || word(bytes, 4)? != cpu
        || word(bytes, 8)? & !0x80000000 != subtype
        || word(bytes, 12)? != 2
    {
        return Err("Expected an Intel or ARM64 Mach-O executable".into());
    }
    let commands_end = 32usize
        .checked_add(word(bytes, 20)? as usize)
        .ok_or("Mach-O command overflow")?;
    if commands_end > bytes.len() {
        return Err("Truncated Mach-O load commands".into());
    }
    let mut offset = 32;
    for _ in 0..word(bytes, 16)? {
        let size = word(bytes, offset + 4)? as usize;
        if size < 8 || !size.is_multiple_of(8) {
            return Err("Invalid Mach-O load command".into());
        }
        offset = offset
            .checked_add(size)
            .ok_or("Mach-O load command overflow")?;
        if offset > commands_end {
            return Err("Truncated Mach-O load command".into());
        }
    }
    if offset != commands_end {
        return Err("Mach-O command inventory mismatch".into());
    }
    Ok(())
}

pub fn universal(intel: &[u8], arm64: &[u8]) -> Result<Vec<u8>> {
    thin(intel, 0x01000007, 3)?;
    thin(arm64, 0x0100000c, 0)?;
    let aligned = |size: usize| (size + 0x3fff) & !0x3fff;
    let intel_offset = 0x4000;
    let arm_offset = aligned(intel_offset + intel.len());
    let mut result = vec![0; arm_offset + arm64.len()];
    let header = [
        0xcafebabeu32,
        2,
        0x01000007,
        word(intel, 8)?,
        intel_offset as u32,
        intel.len() as u32,
        14,
        0x0100000c,
        word(arm64, 8)?,
        arm_offset as u32,
        arm64.len() as u32,
        14,
    ];
    for (slot, word) in result.chunks_exact_mut(4).zip(header) {
        slot.copy_from_slice(&word.to_be_bytes());
    }
    result[intel_offset..intel_offset + intel.len()].copy_from_slice(intel);
    result[arm_offset..].copy_from_slice(arm64);
    Ok(result)
}

pub(crate) fn dylibs(bytes: &[u8]) -> Result<std::collections::BTreeMap<String, (u32, u32)>> {
    thin(bytes, word(bytes, 4)?, word(bytes, 8)? & !0x80000000)?;
    let mut offset = 32;
    let mut result = std::collections::BTreeMap::new();
    for _ in 0..word(bytes, 16)? {
        let kind = word(bytes, offset)?;
        let size = word(bytes, offset + 4)? as usize;
        if [0xc, 0x80000018, 0x8000001f, 0x80000023].contains(&kind) {
            let name_offset = word(bytes, offset + 8)? as usize;
            if size < 24 || name_offset < 24 || name_offset >= size {
                return Err("Invalid Mach-O dylib command".into());
            }
            let name = &bytes[offset + name_offset..offset + size];
            let end = name
                .iter()
                .position(|&v| v == 0)
                .ok_or("Unterminated dylib name")?;
            let name =
                std::str::from_utf8(&name[..end]).map_err(|_| "Invalid dylib name encoding")?;
            if result
                .insert(
                    name.into(),
                    (word(bytes, offset + 16)?, word(bytes, offset + 20)?),
                )
                .is_some()
            {
                return Err("Duplicate dylib load command".into());
            }
        }
        offset += size;
    }
    Ok(result)
}

pub fn check_universal(bytes: &[u8]) -> Result<()> {
    let big = |offset| -> Result<u32> {
        Ok(u32::from_be_bytes(
            bytes
                .get(offset..offset + 4)
                .ok_or("Truncated universal header")?
                .try_into()
                .map_err(|_| "Invalid universal header")?,
        ))
    };
    if big(0)? != 0xcafebabe || big(4)? != 2 {
        return Err("Expected a universal Intel/ARM64 executable".into());
    }
    let mut ranges = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    for index in 0..2 {
        let record = 8 + index * 20;
        let cpu = big(record)?;
        let subtype = match cpu {
            0x01000007 => 3,
            0x0100000c => 0,
            _ => return Err("Unknown universal architecture".into()),
        };
        if !seen.insert(cpu) || big(record + 4)? & !0x80000000 != subtype {
            return Err("Universal architecture inventory changed".into());
        }
        let offset = big(record + 8)? as usize;
        let size = big(record + 12)? as usize;
        let alignment = big(record + 16)?;
        let end = offset.checked_add(size).ok_or("Universal image overflow")?;
        if alignment > 20
            || offset < 48
            || !offset.is_multiple_of(1 << alignment)
            || end > bytes.len()
        {
            return Err("Invalid universal image range".into());
        }
        let image = &bytes[offset..end];
        thin(image, cpu, subtype)?;
        if word(image, 8)? != big(record + 4)? {
            return Err("Universal subtype differs from its Mach-O image".into());
        }
        ranges.push(offset..end);
    }
    if ranges[0].start < ranges[1].end && ranges[1].start < ranges[0].end {
        return Err("Overlapping universal images".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn executable(cpu: u32, subtype: u32) -> Vec<u8> {
        [0xfeedfacf, cpu, subtype, 2, 0, 0, 0, 0]
            .into_iter()
            .flat_map(u32::to_le_bytes)
            .collect()
    }
    #[test]
    fn universal_preserves_both_images_at_page_aligned_offsets() {
        let intel = executable(0x01000007, 3);
        let arm = executable(0x0100000c, 0);
        let fat = universal(&intel, &arm).unwrap();
        check_universal(&fat).unwrap();
        assert_eq!(&fat[..8], &[0xca, 0xfe, 0xba, 0xbe, 0, 0, 0, 2]);
        assert_eq!(&fat[0x4000..0x4020], intel);
        assert_eq!(&fat[0x8000..], arm);
    }
    #[test]
    fn preserves_intel_lib64_capability_and_accepts_native_lipo_order() {
        let intel = executable(0x01000007, 0x80000003);
        let arm = executable(0x0100000c, 0);
        assert!(dylibs(&intel).unwrap().is_empty());
        let mut fat = universal(&intel, &arm).unwrap();
        assert_eq!(&fat[12..16], &0x80000003u32.to_be_bytes());
        check_universal(&fat).unwrap();
        let first = fat[8..28].to_vec();
        let second = fat[28..48].to_vec();
        fat[8..28].copy_from_slice(&second);
        fat[28..48].copy_from_slice(&first);
        check_universal(&fat).unwrap();
        fat[32..36].copy_from_slice(&3u32.to_be_bytes());
        assert!(check_universal(&fat).is_err());
    }

    #[test]
    fn rejects_swapped_architectures_and_malformed_commands() {
        let intel = executable(0x01000007, 3);
        let arm = executable(0x0100000c, 0);
        assert!(universal(&arm, &intel).is_err());
        let mut truncated = intel.clone();
        truncated[20..24].copy_from_slice(&8u32.to_le_bytes());
        assert!(universal(&truncated, &arm).is_err());
        assert!(universal(b"MZ", &arm).is_err());
    }
}
