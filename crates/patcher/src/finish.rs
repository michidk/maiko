//! Guarded assembly changes. Startup patch documentation lives in startup.json;
//! all other transforms mirror the documented hooks in tools/pipeline.py.
use crate::{
    image::{Image, Result, moved_aliases, rename_moved_aliases},
    prepare::{Files, runtime_sources, text},
};
use regex::Regex;
use serde::Deserialize;

#[derive(Deserialize)]
struct Guard {
    address: u32,
    length: usize,
    sha256: String,
}
#[derive(Deserialize)]
struct Replacement {
    before: String,
    after: String,
}
#[derive(Deserialize)]
struct Hook {
    name: String,
    guards: Vec<Guard>,
    replacements: Vec<Replacement>,
}

fn regex(pattern: &str) -> Result<Regex> {
    Regex::new(pattern).map_err(|error| error.to_string())
}

fn relax(input: &str) -> Result<String> {
    let branch = regex(r"(?m)^(j\w+) short ")?;
    let input = branch.replace_all(input, "$1 ");
    let lines: Vec<_> = input.split_inclusive('\n').collect();
    let mut compact = String::new();
    let mut cursor = 0;
    while cursor < lines.len() {
        let line = lines[cursor];
        let count = lines[cursor..]
            .iter()
            .take_while(|&&item| item == line)
            .count();
        if count > 2 && line.starts_with("db ") {
            compact.push_str(&format!("times {count} {line}"));
        } else {
            for _ in 0..count {
                compact.push_str(line);
            }
        }
        cursor += count;
    }
    let mut output = String::new();
    let mut pending = Vec::new();
    fn flush(output: &mut String, pending: &mut Vec<&str>) {
        if !pending.is_empty() {
            output.push_str("db ");
            output.push_str(&pending.join(","));
            output.push('\n');
            pending.clear();
        }
    }
    for line in compact.split_inclusive('\n') {
        if let Some(value) = line.strip_prefix("db ") {
            pending.push(value.trim());
            if pending.len() == 128 {
                flush(&mut output, &mut pending);
            }
        } else {
            flush(&mut output, &mut pending);
            output.push_str(line);
        }
    }
    flush(&mut output, &mut pending);
    Ok(output)
}

fn split(input: &str, number: usize) -> Result<String> {
    let data = regex(r"^(?:(?:times \d+ )?(?:db|dw|dd|dq)|resb)\b")?;
    let mut output = String::new();
    let mut pending = String::new();
    let mut active = "code";
    for line in input.split_inclusive('\n') {
        if line.trim().is_empty()
            || line.starts_with(';')
            || line.starts_with("align ")
            || line.trim().ends_with(':')
        {
            pending.push_str(line);
            continue;
        }
        let target = if data.is_match(line) { "data" } else { "code" };
        if target != active {
            let name = if target == "data" { "dseg" } else { "cseg" };
            let elf = if target == "data" {
                "noexec write align=4"
            } else {
                "exec nowrite align=16"
            };
            output.push_str(&format!("%ifidn __OUTPUT_FORMAT__, elf64\nsection {name}{number:02} progbits alloc {elf}\n%else\nsection {name}{number:02} {target} align=4\n%endif\n"));
            active = target;
        }
        output.push_str(&pending);
        pending.clear();
        output.push_str(line);
    }
    output.push_str(&pending);
    if active != "code" {
        output.push_str(&format!("%ifidn __OUTPUT_FORMAT__, elf64\nsection cseg{number:02} progbits alloc exec nowrite align=16\n%else\nsection cseg{number:02} code align=16\n%endif\n"));
    }
    Ok(output)
}

pub fn finish(bytes: &[u8], mut files: Files) -> Result<Files> {
    let image = Image::load(bytes)?;
    for (name, value) in &mut files {
        if name.starts_with("seg") && name.ends_with(".inc") {
            *value = relax(std::str::from_utf8(value).map_err(|_| "Invalid assembly encoding")?)?
                .into_bytes();
        }
    }
    let mut sources = runtime_sources();
    // SR emits metadata-dependent declarations before translation. The native
    // declarations replace those afterward, as in the reference pipeline.
    let extern_source = crate::prepare::SOURCES
        .iter()
        .find(|(name, _)| *name == "native-metadata/extern.inc")
        .ok_or("Missing native declarations")?
        .1;
    sources.insert("extern.inc".into(), extern_source.to_vec());
    files.extend(sources);
    let mut main = text(&files, "seg02.inc")?.to_owned();
    let hooks: Vec<Hook> =
        serde_json::from_str(include_str!("startup.json")).map_err(|error| error.to_string())?;
    for hook in hooks {
        for guard in hook.guards {
            image
                .check(guard.address, guard.length, &guard.sha256)
                .map_err(|_| format!("{} original bytes changed", hook.name))?;
        }
        for patch in hook.replacements {
            let before = image.relabel(&patch.before)?;
            if main.matches(&before).count() != 1 {
                return Err(format!("{} assembly inventory changed", hook.name));
            }
            main = main.replacen(&before, &image.relabel(&patch.after)?, 1);
        }
    }
    let ticks = regex(r"(?m)^mov (dword \[ebp-0x[0-9a-f]+\]|eax|esi|edi), 0x46c$")?;
    if ticks.find_iter(&main).count() != 27 {
        return Err("BIOS tick pointer inventory changed".into());
    }
    main = ticks
        .replace_all(&main, "mov $1, rg_bios_ticks")
        .into_owned();
    let raw_data = regex(r"(?m)^(?:db|dw|dd) ")?;
    for address in [0xd933c, 0xd93cc, 0xd945c, 0xd94ec, 0xd9578, 0xd9604] {
        let label = format!("loc_{:X}:\n", image.address(address)?);
        if main.matches(&label).count() != 1 {
            return Err(format!("Missing clipping helper {address:#x}"));
        }
        let body = main
            .split_once(&label)
            .and_then(|(_, rest)| rest.split_once("\nRET"))
            .map(|(body, _)| body)
            .ok_or("Missing clipping return")?;
        if raw_data.is_match(body)
            || !body.contains("sub r11d, 0x4")
            || !body.contains(&image.relabel("CALL loc_D92D0")?)
        {
            return Err(format!("Untranslated clipping helper {address:#x}"));
        }
    }
    for (address, end, target) in [
        (0xf1f80, 0xf1ff0, "host_audio_sample_pan"),
        (0xf2010, 0xf2090, "host_audio_stream_release"),
        (0xf2090, 0xf20c0, "host_audio_sample_end"),
        (0xf20c0, 0xf22b0, "host_audio_stream_allocate"),
        (0xf22b0, 0xf2340, "host_audio_sample_type"),
        (0xf2340, 0xf2390, "loc_F2390"),
        (0xf2390, 0xf24a0, "host_audio_stream_load"),
    ] {
        let start = format!("loc_{:X}:\n", image.address(address)?);
        let end = format!("loc_{:X}:\n", image.address(end)?);
        let target = image.relabel(target)?;
        if main.matches(&start).count() != 1 || main.matches(&end).count() != 1 {
            return Err("Smacker callback inventory changed".into());
        }
        let body = main
            .split_once(&start)
            .and_then(|(_, rest)| rest.split_once(&end))
            .and_then(|(body, _)| body.rsplit_once("\nRET"))
            .map(|(body, _)| body)
            .ok_or("Missing Smacker callback return")?;
        if raw_data.is_match(body)
            || !body.contains("r11d")
            || !body.contains(&format!("CALL {target}"))
        {
            return Err(format!(
                "Smacker callback translation failed at {address:#x}"
            ));
        }
    }
    let mut declarations = String::new();
    for index in 0..124 {
        declarations.push_str(&format!("extern host_glide_{index}\n"));
    }
    declarations
        .push_str("extern rg_bios_ticks\nextern host_startup_clock_pace\nextern rg_load_save\n");
    let mut traces = String::new();
    for address in [
        0x1115c2, 0x113c04, 0x113c63, 0x113cad, 0xf1cde, 0x129dc4, 0x119b86, 0x133707, 0x11fd96,
        0x1303d4, 0x130c8b, 0x110826, 0xc7f20, 0xf976c, 0xc80f8, 0x20000, 0x20095, 0x98673,
        0x138b82, 0x2bd78, 0xd92d0,
    ] {
        // The trace service keeps the reference address so logs read the same for every build.
        let label = format!("loc_{:X}:", image.address(address)?);
        if main.matches(&label).count() != 1 {
            return Err(format!("Missing native trace root {label}"));
        }
        main = main.replacen(&label, &format!("{label}\nCALL trace_{address:X}"), 1);
        declarations.push_str(&format!("extern trace_{address:X}\n"));
        traces.push_str(&format!(
            "HOST_SERVICE trace_{address:X}, {}\n",
            0x80000000u32 | address
        ));
    }
    files.insert("seg02.inc".into(), main.into_bytes());
    files
        .entry("extern.inc".into())
        .or_default()
        .extend_from_slice(declarations.as_bytes());
    files.insert("native_traces.inc".into(), traces.into_bytes());
    for number in [5, 7] {
        let name = format!("seg{number:02}.inc");
        let output = split(text(&files, &name)?, number)?;
        files.insert(name, output.into_bytes());
    }
    for name in ["call_trace.inc", "image_variant.inc", "font_metric.inc"] {
        let value = files.get(name).ok_or("Missing macro source")?.clone();
        files
            .entry("macros.inc".into())
            .or_default()
            .extend_from_slice(&value);
    }
    files.remove("RGFX.EXE");
    // Build 5.0: give exported names that moved their reference-image names again.
    let moved = moved_aliases(image.build)?;
    if !moved.is_empty() {
        for (name, bytes) in &mut files {
            if name.starts_with("seg") && name.ends_with(".inc") || name == "rgfx.asm" {
                let text = std::str::from_utf8(bytes).map_err(|_| "Invalid assembly encoding")?;
                *bytes = rename_moved_aliases(&moved, text)?.into_bytes();
            }
        }
    }
    Ok(files)
}
