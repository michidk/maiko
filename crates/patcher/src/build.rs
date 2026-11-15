//! Commands for the pinned WASI assembler. The host supplies execution only.
use crate::kit::Target;

pub fn assembly(target: Target) -> Vec<(Vec<String>, String)> {
    if target == Target::Macos {
        return Vec::new();
    }
    crate::link::engine(target)
        .into_iter()
        .map(|(source, output)| {
            let mut args = vec![
                "nasm".into(),
                "-f".into(),
                target.assembly_format().into(),
                "-Isupport/".into(),
                "-I./".into(),
            ];
            if source == "rgfx" {
                args.push("-O1".into());
            }
            args.extend([format!("{source}.asm"), "-o".into(), output.clone()]);
            (args, output)
        })
        .collect()
}
