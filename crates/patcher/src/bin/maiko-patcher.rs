use maiko_patcher::prepare::Options;
use std::{env, path::Path};

fn run() -> Result<(), String> {
    let args: Vec<_> = env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("build") => {
            // --v5-fixes is a switch; every other option takes a value.
            let v5_fixes = args[2..].iter().any(|arg| arg == "--v5-fixes");
            let rest: Vec<_> = args[2..].iter().filter(|arg| *arg != "--v5-fixes").collect();
            let mut flags = std::collections::BTreeMap::new();
            for pair in rest.chunks(2) {
                if pair.len() != 2 || !["--input", "--kit", "--target", "--output", "--wasmtime"].contains(&pair[0].as_str()) || flags.insert(pair[0].as_str(), pair[1].as_str()).is_some() { return Err("Expected unique --input, --kit, --target, --output and optional --wasmtime and --v5-fixes arguments".into()); }
            }
            let get = |name| flags.get(name).copied().ok_or_else(|| format!("Missing {name}"));
            maiko_patcher::native::build(maiko_patcher::native::BuildOptions { input: Path::new(get("--input")?), kit: Path::new(get("--kit")?), target: maiko_patcher::kit::Target::parse(get("--target")?)?, output: Path::new(get("--output")?), wasmtime: flags.get("--wasmtime").copied().unwrap_or("wasmtime"), options: Options { v5_fixes } })
        }
        _ => Err("Usage: maiko-patcher build --input RGFX.EXE --target linux|windows --kit KIT.zip --output PACKAGE.zip [--wasmtime PATH] [--v5-fixes]".into())
    }
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
