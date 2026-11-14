use maiko_patcher::{
    finish::finish,
    image::Image,
    prepare::{Files, Options, prepare},
};
use std::{env, fs, path::Path};

fn read_files(directory: &Path) -> Result<Files, String> {
    let mut files = Files::new();
    for entry in fs::read_dir(directory).map_err(|error| error.to_string())? {
        let path = entry.map_err(|error| error.to_string())?.path();
        if path.is_file() {
            files.insert(
                path.file_name()
                    .ok_or("Invalid filename")?
                    .to_str()
                    .ok_or("Invalid filename")?
                    .into(),
                fs::read(&path).map_err(|error| error.to_string())?,
            );
        }
    }
    Ok(files)
}

fn write_files(directory: &Path, files: Files) -> Result<(), String> {
    fs::create_dir_all(directory).map_err(|error| error.to_string())?;
    for (name, data) in files {
        fs::write(directory.join(name), data).map_err(|error| error.to_string())?;
    }
    Ok(())
}

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
        Some("inspect") if args.len() == 3 => {
            let image = Image::load(&maiko_patcher::native::read_limited(Path::new(&args[2]), 32 * 1024 * 1024)?)?;
            println!("Recognized RGFX.EXE: {} ({} LE objects)", image.source_hash, image.objects.len()); Ok(())
        }
        Some(stage @ ("prepare" | "prepare-llasm")) if args.len() == 4 || (args.len() == 5 && args[4] == "--v5-fixes") => {
            if Path::new(&args[3]).exists() { return Err("Prepare needs a new, empty output directory".into()); }
            let input = maiko_patcher::native::read_limited(Path::new(&args[2]), 32 * 1024 * 1024)?;
            let options = Options { v5_fixes: args.len() == 5 };
            write_files(Path::new(&args[3]), if stage == "prepare-llasm" { maiko_patcher::llasm::prepare_input(&input, options)? } else { prepare(&input, options)? })
        }
        Some(stage @ ("finish" | "finish-llasm")) if args.len() == 4 => {
            let directory = Path::new(&args[3]);
            let files = read_files(directory)?;
            write_files(directory, if stage == "finish-llasm" { maiko_patcher::llasm::finish(files)? } else { finish(&maiko_patcher::native::read_limited(Path::new(&args[2]), 32 * 1024 * 1024)?, files)? })
        }
        _ => Err("Usage: maiko-patcher build --input RGFX.EXE --target linux|windows --kit KIT.zip --output PACKAGE.zip [--wasmtime PATH] [--v5-fixes]\n       maiko-patcher inspect RGFX.EXE | prepare[-llasm] RGFX.EXE DIRECTORY [--v5-fixes] | finish[-llasm] RGFX.EXE DIRECTORY".into())
    }
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
