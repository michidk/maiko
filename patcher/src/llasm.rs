//! Checked offset-arena translation inputs and CPU-state service boundaries.
use crate::{
    image::{Build, Image, Result, moved_aliases, rename_moved_aliases},
    prepare::{Files, Options, SOURCES, prepare, replacements, text},
};
use regex::Regex;
use serde::Deserialize;
use std::collections::BTreeSet;

#[derive(Deserialize)]
struct Check {
    address: String,
    length: usize,
    sha256: String,
}

/// Address and length of every entry in a checked replacement table.
fn sites(table: &str) -> Result<Vec<(u32, usize)>> {
    table
        .lines()
        .map(|line| {
            let mut fields = line.splitn(3, ',');
            let address = fields
                .next()
                .and_then(|s| s.strip_prefix("loc_"))
                .ok_or("Invalid LLasm replacement label")?;
            let length = fields
                .next()
                .ok_or("Missing LLasm replacement length")?
                .parse::<usize>()
                .map_err(|_| "Invalid LLasm replacement length")?;
            Ok((
                u32::from_str_radix(address, 16).map_err(|_| "Invalid LLasm address")?,
                length,
            ))
        })
        .collect()
}

pub fn prepare_input(input: &[u8], options: Options) -> Result<Files> {
    let image = Image::load(input)?;
    let mut files = prepare(input, options)?;
    let mut checks = None;
    let mut v5_fixes = None;
    for &(path, bytes) in SOURCES {
        let Some(name) = path.strip_prefix("llasm-metadata/") else {
            continue;
        };
        match name {
            "global_aliases.sci" => {
                let aliases = image.aliases(
                    std::str::from_utf8(bytes).map_err(|_| "Invalid LLasm alias table")?,
                )?;
                files
                    .entry(name.into())
                    .or_default()
                    .extend_from_slice(aliases.as_bytes())
            }
            "instruction_replacements.sci" => {
                files.insert(name.into(), replacements(bytes, &image)?.into_bytes());
            }
            "instruction_checks.json" => checks = Some(bytes),
            "v5_fixes.sci" => v5_fixes = Some(bytes),
            _ if name.ends_with(".sci") => {
                let table = std::str::from_utf8(bytes).map_err(|_| "Invalid LLasm table")?;
                files.insert(name.into(), image.relabel(table)?.into_bytes());
            }
            _ => {}
        }
    }
    let checks: Vec<Check> = serde_json::from_slice(checks.ok_or("Missing LLasm byte checks")?)
        .map_err(|e| e.to_string())?;
    let table = sites(text(&files, "instruction_replacements.sci")?)?;
    if checks.len() != table.len() {
        return Err("LLasm replacement inventory changed".into());
    }
    for (check, &(address, length)) in checks.iter().zip(&table) {
        let expected_address = u32::from_str_radix(check.address.trim_start_matches("0x"), 16)
            .map_err(|_| "Invalid LLasm check address")?;
        let checked = image
            .check(expected_address, check.length, &check.sha256)
            .map_err(|_| format!("LLasm replacement byte check failed at {address:#x}"))?;
        if (checked, check.length) != (address, length) {
            return Err(format!(
                "LLasm replacement byte check failed at {address:#x}"
            ));
        }
    }
    // Every replacement on the register-based backend needs a corresponding
    // CPU-state implementation, including sites whose replacement length differs.
    let native = SOURCES
        .iter()
        .find(|(name, _)| *name == "native-metadata/instruction_replacements.sci")
        .ok_or("Missing native replacement inventory")?
        .1;
    for line in replacements(native, &image)?.lines() {
        let address = line
            .split(',')
            .next()
            .and_then(|s| s.strip_prefix("loc_"))
            .and_then(|s| u32::from_str_radix(s, 16).ok())
            .ok_or("Invalid native replacement")?;
        if !table.iter().any(|entry| entry.0 == address) {
            return Err(format!("LLasm is missing native replacement {address:#x}"));
        }
    }
    if options.v5_fixes {
        let llasm = replacements(v5_fixes.ok_or("Missing LLasm version 5 fixes")?, &image)?;
        let native = SOURCES
            .iter()
            .find(|(name, _)| *name == "native-metadata/v5_fixes.sci")
            .ok_or("Missing native version 5 fixes")?
            .1;
        // Both backends must change exactly the same original bytes.
        if sites(&llasm)? != sites(&replacements(native, &image)?)? {
            return Err("LLasm and native version 5 fixes differ".into());
        }
        files
            .get_mut("instruction_replacements.sci")
            .ok_or("Missing LLasm replacements")?
            .extend_from_slice(llasm.as_bytes());
    }
    Ok(files)
}

pub fn finish(mut files: Files, build: Build) -> Result<Files> {
    // Build 5.0: give exported names that moved their reference-image names again.
    let moved = moved_aliases(build)?;
    if !moved.is_empty() {
        for (name, bytes) in &mut files {
            if name.ends_with(".llasm") || name.ends_with(".llinc") {
                let text = std::str::from_utf8(bytes).map_err(|_| "Invalid LLasm encoding")?;
                *bytes = rename_moved_aliases(&moved, text)?.into_bytes();
            }
        }
    }
    files.insert("extern.llinc".into(), b"funcv rg_llasm_service $, service\nfunc rg_llasm_target $\nfunc rg_llasm_variant\nfuncv rg_translation_trap address\nextern rg_bios_ticks\nextern rg_skip_intro\nextern rg_load_save\n".to_vec());
    let mut wrappers = String::from(
        "proc rg_dispatch global c_rg_dispatch\ncall rg_llasm_target $\ntcall tmp0\nendp\n",
    );
    let runtime = SOURCES
        .iter()
        .find(|(name, _)| *name == "runtime/native_services.inc")
        .ok_or("Missing native service inventory")?
        .1;
    let services = [
        std::str::from_utf8(runtime).map_err(|_| "Invalid native services")?,
        text(&files, "glide_services.inc")?,
    ]
    .join("\n");
    let service_pattern =
        Regex::new(r"(?m)^HOST_SERVICE (\w+), (\d+)(?:, (\d+))?").map_err(|e| e.to_string())?;
    for service in service_pattern.captures_iter(&services) {
        wrappers.push_str(&format!(
            "proc {} public\ncall rg_llasm_service $, {}\nPOP tmp1\n",
            &service[1], &service[2]
        ));
        if let Some(pop) = service.get(3).filter(|v| v.as_str() != "0") {
            wrappers.push_str(&format!("add esp, esp, {}\n", pop.as_str()));
        }
        wrappers.push_str("tcall tmp1\nendp\n");
    }
    files.insert("host.llinc".into(), wrappers.into_bytes());
    let source = text(&files, "rgfx.llasm")?.trim_end_matches("\ninclude host.llinc\n");
    // Original: SR marks mixed LE objects constant. Replacement: writable data
    // segments, separate from LLVM function text. Effect: writes remain valid
    // without writable executable pages. Why: objects 5 and 7 mix both kinds.
    // Evidence: guarded LE flags in Image::translator_input and native pipeline.
    let source = source
        .replace("datasegment cseg05 constant", "datasegment cseg05")
        .replace("datasegment cseg07 constant", "datasegment cseg07");
    files.insert(
        "rgfx.llasm".into(),
        format!("{source}\ninclude host.llinc\n").into_bytes(),
    );
    let clock = Regex::new(r"(?m)^(;mov (?:dword \[ebp-0x[0-9a-f]+\]|eax|esi|edi), 0x46c\n(?:add tmpadr, ebp, -[0-9]+\n)?)(mov (?:eax|esi|edi|tmp\d), )0x46c$").map_err(|e| e.to_string())?;
    let action = Regex::new(r"(?m)^(ACTION_\w+)\s*$").map_err(|e| e.to_string())?;
    let mut actions = BTreeSet::new();
    let mut clocks = 0;
    for (name, bytes) in &mut files {
        if !name.starts_with("seg") || !name.ends_with("_code.llinc") {
            continue;
        }
        let code = std::str::from_utf8(bytes).map_err(|_| "Invalid LLasm code encoding")?;
        // Original: NASM instruments the clip-code entry at 0xD92D0.
        // Replacement: register-preserving CPU-state service. Effect: same
        // rendering diagnostics. Why: NASM ABI wrappers cannot run on ARM64.
        // Evidence: host service 0x800D92D0 and tests/check_linux_menu.py.
        let entry = "proc rg_clip_code global c_rg_clip_code\n";
        if code.matches(entry).count() > 1 {
            return Err("LLasm clip trace inventory changed".into());
        }
        let code = code.replace(
            entry,
            &format!(
                "{entry}PUSH loc_20000\ncall rg_llasm_service $, 0x800d92d0\nadd esp, esp, 4\n"
            ),
        );
        // Original: 27 instructions load the BIOS clock literal 0x46c.
        // Replacement: pointer-offset address of the host atomic clock.
        // Effect: engine and host share a clock without mapping DOS low memory.
        // Why: Darwin forbids that mapping. Evidence: pipeline.py's same
        // 27-site inventory and original literal comments required by this regex.
        clocks += clock.find_iter(&code).count();
        let code = clock.replace_all(&code, "${1}${2}rg_bios_ticks");
        actions.extend(
            action
                .captures_iter(&code)
                .map(|capture| capture[1].to_owned()),
        );
        *bytes = code.as_bytes().to_vec();
    }
    if clocks != 27 {
        return Err(format!("LLasm BIOS clock inventory changed: {clocks}"));
    }
    files.insert(
        "macros.llinc".into(),
        actions
            .into_iter()
            .map(|name| format!("macro {name}\nendm\n"))
            .collect::<String>()
            .into_bytes(),
    );
    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn translated(clock_count: usize) -> Files {
        Files::from([
            (
                "rgfx.llasm".into(),
                b"datasegment cseg05 constant\ndatasegment cseg07 constant\n".to_vec(),
            ),
            (
                "glide_services.inc".into(),
                b"HOST_SERVICE host_glide_0, 8192, 12\n".to_vec(),
            ),
            (
                "seg01_code.llinc".into(),
                format!(
                    "proc rg_clip_code global c_rg_clip_code\nACTION_TEST\n{}",
                    ";mov eax, 0x46c\nmov eax, 0x46c\n".repeat(clock_count)
                )
                .into_bytes(),
            ),
        ])
    }

    #[test]
    fn emits_cpu_state_boundaries_and_relocates_clock() {
        let files = finish(translated(27), Build::Reference).unwrap();
        assert!(text(&files, "host.llinc").unwrap().contains("proc host_glide_0 public\ncall rg_llasm_service $, 8192\nPOP tmp1\nadd esp, esp, 12\ntcall tmp1\nendp"));
        assert!(
            text(&files, "rgfx.llasm")
                .unwrap()
                .ends_with("\ninclude host.llinc\n")
        );
        assert!(!text(&files, "rgfx.llasm").unwrap().contains("constant"));
        let code = text(&files, "seg01_code.llinc").unwrap();
        assert_eq!(code.matches("mov eax, rg_bios_ticks").count(), 27);
        assert_eq!(code.matches(";mov eax, 0x46c").count(), 27);
        assert!(
            code.contains("PUSH loc_20000\ncall rg_llasm_service $, 0x800d92d0\nadd esp, esp, 4\n")
        );
        assert_eq!(
            text(&files, "macros.llinc").unwrap(),
            "macro ACTION_TEST\nendm\n"
        );
    }

    #[test]
    fn refuses_incomplete_or_changed_translation() {
        for count in [0, 26, 28] {
            assert!(
                finish(translated(count), Build::Reference)
                    .unwrap_err()
                    .contains("BIOS clock inventory")
            );
        }
        let mut files = translated(27);
        files.remove("rgfx.llasm");
        assert!(
            finish(files, Build::Reference)
                .unwrap_err()
                .contains("Missing rgfx.llasm")
        );
    }
}
