//! Reconstruct a pinned executable from locally assembled guest sections.
//!
//! Templates contain no guest section bytes. The recipe contains section hashes
//! and resolved relocation fields (addresses), never instructions or game data.
use crate::image::{Result, hash};
use object::{Object, ObjectSection};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Recipe {
    pub sections: Vec<Section>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Section {
    pub name: String,
    pub offset: usize,
    pub size: usize,
    pub object_sha256: String,
    pub relocations: Vec<Field>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Field {
    pub offset: usize,
    pub bytes: Vec<u8>,
}
fn guest(name: &str) -> bool {
    (name.starts_with("cseg") || name.starts_with("dseg"))
        && name.len() == 6
        && name[4..].bytes().all(|b| b.is_ascii_digit())
}
fn parse(bytes: &[u8]) -> Result<object::File<'_>> {
    object::File::parse(bytes).map_err(|e| e.to_string())
}
impl Recipe {
    /// Export only when every non-relocation byte agrees with the accepted build.
    /// Guest sections discarded by the native linker have no template slot.
    pub fn export(executable: &[u8], assembled: &[u8]) -> Result<(Self, Vec<u8>)> {
        let binary = parse(executable)?;
        let object = parse(assembled)?;
        if binary.kind() != object::ObjectKind::Executable
            || object.kind() != object::ObjectKind::Relocatable
            || binary.architecture() != object::Architecture::X86_64
            || object.architecture() != object::Architecture::X86_64
        {
            return Err("Expected an x86-64 executable and relocatable engine object".into());
        }
        for section in object.sections() {
            let name = section.name().map_err(|e| e.to_string())?;
            if let object::SectionFlags::Elf { sh_flags } = section.flags()
                && sh_flags & u64::from(object::elf::SHF_ALLOC) == 0
            {
                continue;
            }
            if !guest(name) && ![".pdata", ".xdata"].contains(&name) {
                return Err(format!("Unexpected engine object section {name}"));
            }
        }
        let mut template = executable.to_vec();
        let mut sections = Vec::new();
        for destination in binary.sections() {
            let name = destination.name().map_err(|e| e.to_string())?;
            if !guest(name) {
                continue;
            }
            let source = object
                .section_by_name(name)
                .ok_or_else(|| format!("Missing assembled section {name}"))?;
            let data = source.data().map_err(|e| e.to_string())?;
            let (offset, length) = destination
                .file_range()
                .ok_or("Guest section has no file range")?;
            let offset = usize::try_from(offset).map_err(|e| e.to_string())?;
            // PE file ranges include alignment padding; the object excludes it.
            if data.len() as u64 > length {
                return Err(format!("Section {name} exceeds its template slot"));
            }
            let accepted = executable
                .get(offset..offset + data.len())
                .ok_or("Invalid section range")?;
            let mut expected = data.to_vec();
            let mut relocations = Vec::new();
            for (at, relocation) in source.relocations() {
                let size = usize::from(relocation.size());
                if ![32, 64].contains(&size) {
                    return Err(format!("Unsupported relocation width {size}"));
                }
                let at = usize::try_from(at).map_err(|e| e.to_string())?;
                let end = at.checked_add(size / 8).ok_or("Relocation overflow")?;
                let bytes = accepted
                    .get(at..end)
                    .ok_or("Relocation outside section")?
                    .to_vec();
                expected
                    .get_mut(at..end)
                    .ok_or("Relocation outside object")?
                    .copy_from_slice(&bytes);
                relocations.push(Field { offset: at, bytes });
            }
            if expected != accepted {
                return Err(format!(
                    "Non-relocation bytes differ in {name}: {:?}",
                    expected
                        .iter()
                        .zip(accepted)
                        .enumerate()
                        .filter(|(_, (a, b))| a != b)
                        .take(12)
                        .collect::<Vec<_>>()
                ));
            }
            // Erase the complete file range, including padding. Padding must be zero.
            let end = offset
                .checked_add(length as usize)
                .ok_or("Section overflow")?;
            if template[offset + data.len()..end].iter().any(|b| *b != 0) {
                return Err(format!("Nonzero padding in {name}"));
            }
            template[offset..end].fill(0);
            sections.push(Section {
                name: name.into(),
                offset,
                size: data.len(),
                object_sha256: hash(data),
                relocations,
            });
        }
        if sections.len() < 6 {
            return Err("Executable does not contain the expected guest sections".into());
        }
        let recipe = Self { sections };
        if recipe.apply(&template, assembled)? != executable {
            return Err("Template roundtrip failed".into());
        }
        Ok((recipe, template))
    }
    pub fn apply(&self, template: &[u8], assembled: &[u8]) -> Result<Vec<u8>> {
        let object = parse(assembled)?;
        let mut output = template.to_vec();
        for section in &self.sections {
            if !guest(&section.name) {
                return Err("Invalid guest section name".into());
            }
            let source = object
                .section_by_name(&section.name)
                .ok_or("Missing guest section")?;
            let data = source.data().map_err(|e| e.to_string())?;
            if data.len() != section.size || hash(data) != section.object_sha256 {
                return Err(format!(
                    "Assembled section {} differs from the accepted build",
                    section.name
                ));
            }
            let end = section
                .offset
                .checked_add(section.size)
                .ok_or("Section overflow")?;
            let destination = output
                .get_mut(section.offset..end)
                .ok_or("Section outside template")?;
            if destination.iter().any(|b| *b != 0) {
                return Err("Template contains guest bytes or overlapping sections".into());
            }
            destination.copy_from_slice(data);
            for field in &section.relocations {
                if ![4, 8].contains(&field.bytes.len()) {
                    return Err("Invalid relocation field".into());
                }
                let end = field
                    .offset
                    .checked_add(field.bytes.len())
                    .ok_or("Relocation overflow")?;
                destination
                    .get_mut(field.offset..end)
                    .ok_or("Relocation outside section")?
                    .copy_from_slice(&field.bytes);
            }
        }
        Ok(output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> (Recipe, Vec<u8>, Vec<u8>) {
        let mut object = object::write::Object::new(
            object::BinaryFormat::Elf,
            object::Architecture::X86_64,
            object::Endianness::Little,
        );
        let id = object.add_section(Vec::new(), b"cseg02".to_vec(), object::SectionKind::Text);
        object.append_section_data(id, &[0x90; 16], 1);
        let assembled = object.write().unwrap();
        let recipe = Recipe {
            sections: vec![Section {
                name: "cseg02".into(),
                offset: 8,
                size: 16,
                object_sha256: hash(&[0x90; 16]),
                relocations: vec![Field {
                    offset: 4,
                    bytes: vec![1, 2, 3, 4],
                }],
            }],
        };
        (recipe, vec![0; 32], assembled)
    }
    #[test]
    fn fills_only_verified_guest_slots_and_relocation_fields() {
        let (recipe, template, assembled) = fixture();
        let output = recipe.apply(&template, &assembled).unwrap();
        assert_eq!(&output[..8], &[0; 8]);
        assert_eq!(&output[8..12], &[0x90; 4]);
        assert_eq!(&output[12..16], &[1, 2, 3, 4]);
        assert_eq!(&output[24..], &[0; 8]);
    }
    #[test]
    fn refuses_wrong_translation_guest_bytes_overlap_and_invalid_ranges() {
        let (mut recipe, mut template, assembled) = fixture();
        template[8] = 1;
        assert!(recipe.apply(&template, &assembled).is_err());
        template[8] = 0;
        recipe.sections[0].object_sha256 = "0".repeat(64);
        assert!(recipe.apply(&template, &assembled).is_err());
        recipe.sections[0].object_sha256 = hash(&[0x90; 16]);
        recipe.sections[0].relocations[0].offset = usize::MAX;
        assert!(recipe.apply(&template, &assembled).is_err());
        recipe.sections[0].offset = usize::MAX;
        assert!(recipe.apply(&template, &assembled).is_err());
    }
}
