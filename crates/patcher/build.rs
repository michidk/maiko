use std::{env, fs, path::Path};
fn main() {
    let root = Path::new("../..");
    let mut source = String::from("pub const SOURCES: &[(&str, &[u8])] = &[\n");
    // Embedded under the names the patcher looks them up by.
    for (folder, directory) in [
        ("metadata/image", "metadata"),
        ("metadata/native", "native-metadata"),
        ("metadata/llasm", "llasm-metadata"),
        ("metadata/build5", "build5-metadata"),
        ("crates/runtime", "runtime"),
    ] {
        let mut entries: Vec<_> = fs::read_dir(root.join(folder))
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .collect();
        entries.sort();
        for path in entries {
            if path.is_file()
                && path.extension().is_some_and(|extension| {
                    matches!(extension.to_str(), Some("sci" | "inc" | "asm"))
                        || (directory == "llasm-metadata" && extension == "json")
                })
            {
                println!("cargo:rerun-if-changed={}", path.display());
                let name = format!(
                    "{directory}/{}",
                    path.file_name().unwrap().to_str().unwrap()
                );
                let full = fs::canonicalize(&path).unwrap();
                source.push_str(&format!(
                    "({:?}, include_bytes!({:?})),\n",
                    name,
                    full.to_str().unwrap()
                ));
            }
        }
    }
    source.push_str("];\n");
    fs::write(
        Path::new(&env::var("OUT_DIR").unwrap()).join("sources.rs"),
        source,
    )
    .unwrap();
}
