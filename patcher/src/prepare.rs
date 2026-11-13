//! Prepare the same guarded SR inputs as tools/pipeline.py without host tools.
use crate::image::{Image, Result, hash};
use regex::bytes::Regex;
use std::collections::BTreeMap;

include!(concat!(env!("OUT_DIR"), "/sources.rs"));
pub type Files = BTreeMap<String, Vec<u8>>;

pub fn text<'a>(files: &'a Files, name: &str) -> Result<&'a str> {
    std::str::from_utf8(files.get(name).ok_or_else(|| format!("Missing {name}"))?)
        .map_err(|_| format!("Invalid UTF-8 in {name}"))
}

fn append(files: &mut Files, name: &str, value: &str) {
    files
        .entry(name.into())
        .or_default()
        .extend_from_slice(value.as_bytes());
}

fn replacements(bytes: &[u8], image: &Image) -> Result<String> {
    let source = std::str::from_utf8(bytes).map_err(|_| "Invalid replacement table encoding")?;
    let mut documentation = String::new();
    let mut digest = None;
    let mut output = String::new();
    for line in source.lines() {
        if line.trim().is_empty() {
            documentation.clear();
        } else if let Some(comment) = line.strip_prefix('#') {
            if let Some(value) = comment.trim().strip_prefix("sha256:") {
                digest = Some(value.trim());
            } else {
                documentation.push_str(comment);
                documentation.push('\n');
            }
        } else {
            for field in ["Original:", "Replacement:", "Effect:", "Why:", "Evidence:"] {
                if !documentation.contains(field) {
                    return Err(format!("Replacement lacks {field}: {line}"));
                }
            }
            let mut parts = line.splitn(3, ',');
            let label = parts.next().ok_or("Missing replacement label")?;
            let address =
                u32::from_str_radix(label.strip_prefix("loc_").ok_or("Invalid label")?, 16)
                    .map_err(|_| "Invalid replacement address")?;
            let len = parts
                .next()
                .ok_or("Missing replacement length")?
                .parse()
                .map_err(|_| "Invalid replacement length")?;
            if parts.next().is_none() {
                return Err("Missing replacement text".into());
            }
            if Some(hash(image.at(address, len)?)).as_deref() != digest.take() {
                return Err(format!("Original bytes changed at {label}"));
            }
            output.push_str(line);
            output.push('\n');
        }
    }
    Ok(output)
}

pub fn prepare(bytes: &[u8]) -> Result<Files> {
    let image = Image::load(bytes)?;
    let mut files = Files::new();
    for (path, data) in SOURCES
        .iter()
        .filter(|(path, _)| path.starts_with("metadata/"))
    {
        let name = path.split('/').next_back().ok_or("Invalid embedded path")?;
        let data = if name == "instruction_replacements.sci" {
            replacements(data, &image)?.into_bytes()
        } else {
            data.to_vec()
        };
        files.insert(name.into(), data);
    }
    for address in [0xf70b6, 0xf70eb, 0x119cde, 0x13b3d7, 0x13b461] {
        if image.at(address, 4)? != [0x66, 0xa9, 0x89, 0xcf] {
            return Err(format!("Overlapping instruction changed at {address:#x}"));
        }
    }
    let object = image.objects.get(1).ok_or("Missing code object")?;
    let regex = Regex::new(r"_[A-Z][A-Z0-9_]+@[0-9]+\x00").map_err(|error| error.to_string())?;
    let names: Vec<_> = regex.find_iter(&object.data).collect();
    if names.len() != 130 {
        return Err("Unexpected Glide import-name inventory".into());
    }
    files.insert(
        "fixup_do_not_interpret_as_code.sci".into(),
        names
            .iter()
            .map(|name| format!("loc_{:X}\n", object.base as usize + name.start()))
            .collect::<String>()
            .into_bytes(),
    );
    for (path, data) in SOURCES
        .iter()
        .filter(|(path, _)| path.starts_with("native-metadata/"))
    {
        let name = path.split('/').next_back().ok_or("Invalid embedded path")?;
        match name {
            "instruction_replacements.sci" => {
                append(&mut files, name, &replacements(data, &image)?)
            }
            "global_aliases.sci" | "fixup_do_not_interpret_as_code.sci" => files
                .entry(name.into())
                .or_default()
                .extend_from_slice(data),
            _ => {
                files.insert(name.into(), data.to_vec());
            }
        }
    }
    // This range contains lookup data. SR's exact-target no-code list must
    // include each address, matching the existing pipeline's exclusion.
    for address in 0x160000..0x163000 {
        append(
            &mut files,
            "fixup_do_not_interpret_as_code.sci",
            &format!("loc_{address:X}\n"),
        );
    }
    for index in 0..124u32 {
        let address = 0x13d988 + index * 5;
        let code = image.at(address, 5)?;
        let displacement =
            i32::from_le_bytes(code[1..].try_into().map_err(|_| "Invalid Glide call")?);
        if code[0] != 0xe8 || i64::from(address) + 5 + i64::from(displacement) != 0x128c60 {
            return Err(format!("Glide stub changed at {address:#x}"));
        }
        let pointer = u32::from_le_bytes(
            image
                .at(0x13dc14 + index * 4, 4)?
                .try_into()
                .map_err(|_| "Invalid Glide pointer")?,
        )
        .checked_add(0x20000)
        .ok_or("Glide pointer overflow")?;
        let decorated = image.at(pointer, 128)?;
        let name = std::str::from_utf8(
            decorated
                .split(|&byte| byte == 0)
                .next()
                .ok_or("Missing Glide name")?,
        )
        .map_err(|_| "Invalid Glide name")?;
        let (symbol, count) = name
            .strip_prefix('_')
            .and_then(|value| value.split_once('@'))
            .ok_or("Invalid Glide ABI name")?;
        let bytes: u32 = count.parse().map_err(|_| "Invalid Glide argument count")?;
        if !symbol
            .bytes()
            .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit())
            || !bytes.is_multiple_of(4)
            || bytes > 64
        {
            return Err("Invalid Glide ABI name".into());
        }
        append(
            &mut files,
            "external_procedures.sci",
            &format!("loc_{address:X},host_glide_{index}\n"),
        );
        append(
            &mut files,
            "glide_services.inc",
            &format!(
                "HOST_SERVICE host_glide_{index}, {}, {bytes}\n",
                0x2000 + index
            ),
        );
        append(
            &mut files,
            "glide_imports.h",
            &format!("#define RG_{symbol} {index}\n"),
        );
    }
    files.insert("RGFX.EXE".into(), image.translator_input()?);
    Ok(files)
}

pub fn runtime_sources() -> Files {
    SOURCES
        .iter()
        .filter_map(|(path, data)| {
            path.strip_prefix("runtime/")
                .map(|name| (name.into(), data.to_vec()))
        })
        .collect()
}
