//! Native host for the same WASI tools used in the browser.
use crate::{
    build::assembly,
    finish::finish,
    image::{Image, Result},
    kit::{Kit, Target, safe_path},
    prepare::{Files, Options, prepare},
};
use std::{fs, io::Write, path::Path, process::Command};

pub fn read_limited(path: &Path, limit: u64) -> Result<Vec<u8>> {
    let file = fs::File::open(path).map_err(|error| error.to_string())?;
    let mut bytes = Vec::new();
    std::io::Read::take(file, limit + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| error.to_string())?;
    if bytes.len() as u64 > limit {
        return Err(format!("{} exceeds the size limit", path.display()));
    }
    Ok(bytes)
}
use std::io::Read;

pub fn write_files(directory: &Path, files: &Files) -> Result<()> {
    for (name, data) in files {
        if !safe_path(name) {
            return Err("Unsafe compiler filename".into());
        }
        let path = directory.join(name);
        fs::create_dir_all(path.parent().ok_or("Invalid compiler filename")?)
            .map_err(|error| error.to_string())?;
        fs::write(path, data).map_err(|error| error.to_string())?;
    }
    Ok(())
}

pub fn read_sources(directory: &Path) -> Result<Files> {
    let mut files = Files::new();
    for entry in fs::read_dir(directory).map_err(|error| error.to_string())? {
        let path = entry.map_err(|error| error.to_string())?.path();
        if path.is_file() {
            let name = path
                .file_name()
                .and_then(|name| name.to_str())
                .ok_or("Invalid compiler filename")?;
            if name.ends_with(".inc")
                || name.ends_with(".asm")
                || name.ends_with(".sci")
                || name.ends_with(".h")
            {
                files.insert(name.into(), read_limited(&path, 256 * 1024 * 1024)?);
            }
        }
    }
    Ok(files)
}

pub struct BuildOptions<'a> {
    pub input: &'a Path,
    pub kit: &'a Path,
    pub target: Target,
    pub output: &'a Path,
    pub wasmtime: &'a str,
    pub options: Options,
}

pub fn build(options: BuildOptions<'_>) -> Result<()> {
    if options.target == Target::Macos {
        return Err("Use the browser builder for Mac packages, or follow platforms/macos/README.md for a local source build".into());
    }
    if options.output.exists() {
        return Err("Output already exists; choose a new ZIP path".into());
    }
    let input = read_limited(options.input, 32 * 1024 * 1024)?;
    Image::load(&input)?;
    let kit = Kit::load(
        &read_limited(options.kit, 768 * 1024 * 1024)?,
        options.target,
    )?;
    let wasmtime = if Path::new(options.wasmtime).components().count() > 1 {
        fs::canonicalize(options.wasmtime).map_err(|e| e.to_string())?
    } else {
        options.wasmtime.into()
    };
    let temporary = tempfile::tempdir().map_err(|error| error.to_string())?;
    let work = temporary.path().join("work");
    fs::create_dir(&work).map_err(|error| error.to_string())?;
    write_files(&work, &kit.compiler_files())?;
    write_files(&work, &prepare(&input, options.options)?)?;
    let tools = temporary.path().join("tools");
    fs::create_dir(&tools).map_err(|error| error.to_string())?;
    for tool in options.target.tools() {
        fs::write(
            tools.join(format!("{tool}.wasm")),
            &kit.files[&format!("tools/{tool}.wasm")],
        )
        .map_err(|error| error.to_string())?;
    }
    // args[0] is the program name the tool sees; LLD selects its driver from it.
    let execute = |tool: &str, args: &[String]| -> Result<()> {
        let result = Command::new(&wasmtime)
            .arg("run")
            .arg("--dir")
            .arg(".")
            .arg("--argv0")
            .arg(&args[0])
            .arg(tools.join(format!("{tool}.wasm")))
            .args(&args[1..])
            .current_dir(&work)
            .status()
            .map_err(|error| {
                format!("Cannot run Wasmtime: {error}. Install Wasmtime or pass --wasmtime PATH.")
            })?;
        if !result.success() {
            return Err(format!("{tool} failed: {result}"));
        }
        Ok(())
    };
    eprintln!("Translating RGFX.EXE locally...");
    execute("sr", &["sr".into(), "RGFX.EXE".into(), "rgfx.asm".into()])?;
    write_files(&work, &finish(&input, read_sources(&work)?)?)?;
    fs::remove_file(work.join("RGFX.EXE")).map_err(|error| error.to_string())?;
    for (args, _) in assembly(options.target) {
        eprintln!("Assembling {}...", args[args.len() - 3]);
        execute("nasm", &args)?;
    }
    eprintln!("Linking the native executable...");
    write_files(&work, &crate::link::files(&kit))?;
    let (args, output) = crate::link::command(options.target)?;
    execute("lld", &args)?;
    let executable = read_limited(&work.join(output), 256 * 1024 * 1024)?;
    let package = kit.package(&executable)?;
    let mut output = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(options.output)
        .map_err(|error| error.to_string())?;
    if let Err(error) = output.write_all(&package).and_then(|()| output.sync_all()) {
        drop(output);
        let _ = fs::remove_file(options.output);
        return Err(error.to_string());
    }
    eprintln!("Created {}", options.output.display());
    Ok(())
}
