//! Export a source-free template from a tested native build.
use maiko_patcher::{
    image::hash,
    kit::{Manifest, Target, zip},
    native::read_limited,
    prepare::Files,
    relink::Recipe,
};
use std::{
    env, fs,
    io::Write,
    path::Path,
    process::{Command, Stdio},
};
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
#[path = "maiko-kit/macos_export.rs"]
mod macos_export;
#[path = "maiko-kit/macos_tools.rs"]
mod macos_tools;
#[path = "maiko-kit/tools.rs"]
mod tools;
const SR: &str = "ac690ddf3010bc3d2c875cb1b5e8da4f53a8d5b3";
fn run(command: &mut Command) -> Result<Vec<u8>> {
    eprintln!("Running {command:?}");
    let result = command.stderr(Stdio::inherit()).output()?;
    if !result.status.success() {
        return Err(format!("Command failed: {command:?}: {}", result.status).into());
    }
    Ok(result.stdout)
}
fn download(url: &str, path: &Path, expected: &str) -> Result<()> {
    if !path.exists() {
        run(Command::new("curl")
            .args(["--fail", "--location", "--retry", "2", "--output"])
            .arg(path)
            .arg(url))?;
    }
    if hash(&fs::read(path)?) != expected {
        return Err(format!("Checksum mismatch: {}", path.display()).into());
    }
    Ok(())
}
fn archive(repo: &Path, revision: &str, directory: &Path) -> Result<()> {
    fs::create_dir_all(directory)?;
    let bytes = run(Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(["archive", revision]))?;
    let mut process = Command::new("tar")
        .arg("-x")
        .arg("-C")
        .arg(directory)
        .stdin(Stdio::piped())
        .spawn()?;
    process
        .stdin
        .take()
        .ok_or("Missing tar stdin")?
        .write_all(&bytes)?;
    if !process.wait()?.success() {
        return Err("Cannot extract pinned source".into());
    }
    Ok(())
}
fn patch(directory: &Path, name: &str) -> Result<()> {
    let mut process = Command::new("patch")
        .args(["-p1", "-d"])
        .arg(directory)
        .stdin(Stdio::piped())
        .spawn()?;
    process
        .stdin
        .take()
        .ok_or("Missing patch stdin")?
        .write_all(&fs::read(name)?)?;
    if !process.wait()?.success() {
        return Err(format!("Cannot apply {name}").into());
    }
    Ok(())
}

fn export(
    target: Target,
    executable: &Path,
    object: &Path,
    runtime: &Path,
    revision: &str,
    output: &Path,
) -> Result<()> {
    if output.exists() {
        return Err("Output already exists".into());
    }
    let executable = read_limited(executable, 256 * 1024 * 1024)?;
    let assembled = read_limited(object, 256 * 1024 * 1024)?;
    let (recipe, template) = Recipe::export(&executable, &assembled)?;
    let mut files = Files::new();
    files.insert("template.bin".into(), template);
    files.insert("recipe.json".into(), serde_json::to_vec(&recipe)?);
    for tool in ["sr", "nasm"] {
        files.insert(
            format!("tools/{tool}.wasm"),
            fs::read(format!(".cache/client-toolchain/{tool}.wasm"))?,
        );
    }
    files.insert(
        "tools/LICENSES.txt".into(),
        fs::read(".cache/client-toolchain/LICENSES.txt")?,
    );
    let support = String::from_utf8(run(Command::new("git").args([
        "-C",
        ".cache/sr",
        "ls-tree",
        "-r",
        "--name-only",
        SR,
        "SR/x64-support",
    ]))?)?;
    for path in support
        .lines()
        .filter(|p| p.ends_with(".inc") || p.ends_with(".asm"))
    {
        let name = Path::new(path)
            .file_name()
            .ok_or("Invalid support name")?
            .to_string_lossy();
        files.insert(
            format!("support/{name}"),
            run(Command::new("git").args(["-C", ".cache/sr", "show", &format!("{SR}:{path}")]))?,
        );
    }
    // This directory is deliberately curated by the maintainer. Never copy a
    // build directory recursively: it contains proprietary translated output.
    let mut package_files = Vec::new();
    for entry in fs::read_dir(runtime)? {
        let path = entry?.path();
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or("Invalid runtime name")?;
        if !path.is_file()
            || !(name.ends_with(".dll")
                || name.ends_with(".so")
                || name.contains(".so.")
                || name.ends_with(".txt")
                || name.ends_with(".tar.gz"))
        {
            return Err(format!("Unexpected curated runtime file: {name}").into());
        }
        files.insert(
            format!("runtime/{name}"),
            read_limited(&path, 128 * 1024 * 1024)?,
        );
        package_files.push(name.to_owned());
    }
    package_files.sort();
    let manifest = Manifest {
        format: 1,
        target,
        revision: revision.into(),
        executable_sha256: hash(&executable),
        files: files.iter().map(|(n, b)| (n.clone(), hash(b))).collect(),
        package_files,
        package_links: Default::default(),
    };
    files.insert(
        "manifest.json".into(),
        serde_json::to_vec_pretty(&manifest)?,
    );
    let bytes = zip(&files, None)?;
    // Repeat the consumer validation before exporting anything.
    let kit = maiko_patcher::kit::Kit::load(&bytes, target)?;
    kit.package(&kit.relink(&assembled)?)?;
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output)?;
    file.write_all(&bytes)?;
    println!("{}  {}", hash(&bytes), output.display());
    Ok(())
}
fn web_tools(kit: &Path, output: &Path) -> Result<()> {
    let kit = maiko_patcher::kit::Kit::load(&read_limited(kit, 768 * 1024 * 1024)?, Target::Macos)?;
    fs::create_dir_all(output)?;
    fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output.join("dlang.js"))?
        .write_all(
            kit.files
                .get("tools/dlang.js")
                .ok_or("Missing D compiler driver")?,
        )?;
    Ok(())
}

fn main() -> Result<()> {
    let args: Vec<_> = env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("tools") if args.len() == 2 => tools::build(),
        Some("tools-macos") if args.len() == 2 => macos_tools::build(),
        Some("web-tools") if args.len() == 4 => web_tools(Path::new(&args[2]), Path::new(&args[3])),
        Some("export-macos") if args.len() == 7 => macos_export::export(Path::new(&args[2]), Path::new(&args[3]), Path::new(&args[4]), &args[5], Path::new(&args[6])),
        Some("export") if args.len() == 8 => export(Target::parse(&args[2])?, Path::new(&args[3]), Path::new(&args[4]), Path::new(&args[5]), &args[6], Path::new(&args[7])),
        _ => Err("Usage: maiko-kit tools | tools-macos | web-tools MACOS_KIT DIST_DIRECTORY | export TARGET ACCEPTED_EXECUTABLE ENGINE_OBJECT CURATED_RUNTIME REVISION OUTPUT.zip | export-macos CHECKED_BUILD CURATED_RUNTIME ACCEPTED_UNIVERSAL REVISION OUTPUT.zip".into()),
    }
}
