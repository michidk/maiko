#[cfg(not(target_arch = "wasm32"))]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = std::env::args_os().skip(1).collect::<Vec<_>>();
    if args.len() != 2 {
        return Err("Usage: maiko-macos-package STAGED_RUNTIME OUTPUT.zip".into());
    }
    maiko_patcher::macos_package::package(
        std::path::Path::new(&args[0]),
        std::path::Path::new(&args[1]),
    )?;
    Ok(())
}
#[cfg(target_arch = "wasm32")]
fn main() {}
